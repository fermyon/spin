use std::marker::PhantomData;

use anyhow::Context;

use crate::Factor;

// TODO(lann): Most of the unsafe shenanigans here probably aren't worth it;
// consider replacing with e.g. `Any::downcast`.

/// Implemented by `#[derive(RuntimeFactors)]`
pub trait RuntimeFactors: Sized {
    type AppState;
    type InstancePreparers;
    type InstanceState: Send + 'static;

    #[doc(hidden)]
    unsafe fn instance_preparer_offset<F: Factor>() -> Option<usize>;

    #[doc(hidden)]
    unsafe fn instance_state_offset<F: Factor>() -> Option<usize>;

    fn app_config<F: Factor>(app_configs: &Self::AppState) -> Option<&F::AppState>;

    fn instance_state_getter<F: Factor>() -> Option<Getter<Self::InstanceState, F::InstanceState>> {
        let offset = unsafe { Self::instance_state_offset::<F>()? };
        Some(Getter {
            offset,
            _phantom: PhantomData,
        })
    }

    fn instance_state_getter2<F1: Factor, F2: Factor>(
    ) -> Option<Getter2<Self::InstanceState, F1::InstanceState, F2::InstanceState>> {
        let offset1 = unsafe { Self::instance_state_offset::<F1>()? };
        let offset2 = unsafe { Self::instance_state_offset::<F2>()? };
        assert_ne!(
            offset1, offset2,
            "instance_state_getter2 with same factor twice would alias"
        );
        Some(Getter2 {
            offset1,
            offset2,
            _phantom: PhantomData,
        })
    }

    fn instance_preparer_mut<F: Factor>(
        preparers: &mut Self::InstancePreparers,
    ) -> crate::Result<Option<&mut F::InstancePreparer>> {
        unsafe {
            let offset = Self::instance_preparer_offset::<F>().context("no such factor")?;
            let ptr = preparers as *mut Self::InstancePreparers;
            let opt = &mut *ptr.add(offset).cast::<Option<F::InstancePreparer>>();
            Ok(opt.as_mut())
        }
    }
}

pub struct Getter<T, U> {
    pub(crate) offset: usize,
    pub(crate) _phantom: PhantomData<fn(&mut T) -> &mut U>,
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
    pub(crate) offset1: usize,
    pub(crate) offset2: usize,
    #[allow(clippy::type_complexity)]
    pub(crate) _phantom: PhantomData<fn(&mut T) -> (&mut U, &mut V)>,
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
