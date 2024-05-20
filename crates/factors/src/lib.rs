use std::{any::Any, marker::PhantomData};

use spin_app::App;
pub use spin_factors_derive::SpinFactors;

pub use wasmtime;

pub type Error = wasmtime::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub type Linker<Factors> = wasmtime::component::Linker<<Factors as SpinFactors>::Data>;
pub type ModuleLinker<Factors> = wasmtime::Linker<<Factors as SpinFactors>::Data>;

pub trait Factor: Any + Sized {
    type Builder: FactorBuilder<Self>;
    type Data;

    /// Initializes this Factor for a runtime. This will be called exactly once
    fn init<Factors: SpinFactors>(&mut self, mut ctx: InitContext<Factors, Self>) -> Result<()> {
        _ = &mut ctx;
        Ok(())
    }

    fn module_init<Factors: SpinFactors>(
        &mut self,
        mut ctx: ModuleInitContext<Factors, Self>,
    ) -> Result<()> {
        _ = &mut ctx;
        Ok(())
    }

    fn validate_app(&self, app: &App) -> Result<()> {
        _ = app;
        Ok(())
    }
}

pub struct FactorInitContext<'a, Factors: SpinFactors, Fact: Factor, Linker> {
    linker: &'a mut Linker,
    get_data: fn(&mut Factors::Data) -> &mut Fact::Data,
}

pub type InitContext<'a, Factors, Fact> = FactorInitContext<'a, Factors, Fact, Linker<Factors>>;

pub type ModuleInitContext<'a, Factors, Fact> =
    FactorInitContext<'a, Factors, Fact, ModuleLinker<Factors>>;

impl<'a, Factors: SpinFactors, Fact: Factor, Linker> FactorInitContext<'a, Factors, Fact, Linker> {
    #[doc(hidden)]
    pub fn new(
        linker: &'a mut Linker,
        get_data: fn(&mut Factors::Data) -> &mut Fact::Data,
    ) -> Self {
        Self { linker, get_data }
    }

    pub fn linker(&mut self) -> &mut Linker {
        self.linker
    }

    pub fn link_bindings(
        &mut self,
        add_to_linker: impl Fn(&mut Linker, fn(&mut Factors::Data) -> &mut Fact::Data) -> Result<()>,
    ) -> Result<()>
where {
        add_to_linker(self.linker, self.get_data)
    }
}

impl<'a, Factors: SpinFactors> PrepareContext<'a, Factors> {
    pub fn builder_mut<T: Factor>(&mut self) -> Result<&mut T::Builder> {
        let err_msg = match Factors::builder_mut::<T>(self.builders) {
            Some(Some(builder)) => return Ok(builder),
            Some(None) => "builder not yet prepared",
            None => "no such factor",
        };
        Err(Error::msg(format!(
            "could not get builder for {ty}: {err_msg}",
            ty = std::any::type_name::<T>()
        )))
    }
}

/// Implemented by `#[derive(SpinFactors)]`
pub trait SpinFactors: Sized {
    type Builders;
    type Data: Send + 'static;

    #[doc(hidden)]
    unsafe fn factor_builder_offset<T: Factor>() -> Option<usize>;

    #[doc(hidden)]
    unsafe fn factor_data_offset<T: Factor>() -> Option<usize>;

    fn data_getter<T: Factor>() -> Option<Getter<Self::Data, T::Data>> {
        let offset = unsafe { Self::factor_data_offset::<T>()? };
        Some(Getter {
            offset,
            _phantom: PhantomData,
        })
    }

    fn data_getter2<T1: Factor, T2: Factor>() -> Option<Getter2<Self::Data, T1::Data, T2::Data>> {
        let offset1 = unsafe { Self::factor_data_offset::<T1>()? };
        let offset2 = unsafe { Self::factor_data_offset::<T2>()? };
        assert_ne!(
            offset1, offset2,
            "data_getter2 with same factor twice would alias"
        );
        Some(Getter2 {
            offset1,
            offset2,
            _phantom: PhantomData,
        })
    }

    fn builder_mut<T: Factor>(builders: &mut Self::Builders) -> Option<Option<&mut T::Builder>> {
        unsafe {
            let offset = Self::factor_builder_offset::<T>()?;
            let ptr = builders as *mut Self::Builders;
            let opt = &mut *ptr.add(offset).cast::<Option<T::Builder>>();
            Some(opt.as_mut())
        }
    }
}

pub struct Getter<T, U> {
    offset: usize,
    _phantom: PhantomData<fn(&mut T) -> &mut U>,
}

impl<T, U> Getter<T, U> {
    pub fn get_mut<'a>(&self, container: &'a mut T) -> &'a mut U {
        let ptr = container as *mut T;
        unsafe { &mut *ptr.add(self.offset).cast::<U>() }
    }
}

impl<T, U> Clone for Getter<T, U> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, U> Copy for Getter<T, U> {}

pub struct Getter2<T, U, V> {
    offset1: usize,
    offset2: usize,
    #[allow(clippy::type_complexity)]
    _phantom: PhantomData<fn(&mut T) -> (&mut U, &mut V)>,
}

impl<T, U, V> Getter2<T, U, V> {
    pub fn get_mut<'a>(&self, container: &'a mut T) -> (&'a mut U, &'a mut V)
    where
        T: 'static,
        U: 'static,
        V: 'static,
    {
        let ptr = container as *mut T;
        unsafe {
            (
                &mut *ptr.add(self.offset1).cast::<U>(),
                &mut *ptr.add(self.offset2).cast::<V>(),
            )
        }
    }
}

impl<T, U, V> Clone for Getter2<T, U, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, U, V> Copy for Getter2<T, U, V> {}

pub trait FactorBuilder<T: Factor>: Sized {
    fn prepare<Factors: SpinFactors>(_factor: &T, _ctx: PrepareContext<Factors>) -> Result<Self>;

    fn build(self) -> Result<T::Data>;
}

pub struct PrepareContext<'a, Factors: SpinFactors> {
    builders: &'a mut Factors::Builders,
    // TODO: component: &'a AppComponent,
}

impl<'a, Factors: SpinFactors> PrepareContext<'a, Factors> {
    #[doc(hidden)]
    pub fn new(builders: &'a mut Factors::Builders) -> Self {
        Self { builders }
    }
}

pub type DefaultBuilder = ();

impl<T: Factor> FactorBuilder<T> for DefaultBuilder
where
    T::Data: Default,
{
    fn prepare<Factors: SpinFactors>(factor: &T, ctx: PrepareContext<Factors>) -> Result<Self> {
        (_, _) = (factor, ctx);
        Ok(())
    }

    fn build(self) -> Result<T::Data> {
        Ok(Default::default())
    }
}
