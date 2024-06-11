use std::{any::TypeId, marker::PhantomData};

use crate::{factor::FactorInstanceState, Factor};

/// Implemented by `#[derive(RuntimeFactors)]`
pub trait RuntimeFactors: Sized + 'static {
    type AppState;
    type InstanceBuilders;
    type InstanceState: Send + 'static;

    fn app_state<F: Factor>(app_state: &Self::AppState) -> Option<&F::AppState>;

    fn instance_builder_mut<F: Factor>(
        builders: &mut Self::InstanceBuilders,
    ) -> Option<Option<&mut F::InstanceBuilder>>;

    #[doc(hidden)]
    fn instance_state_offset<F: Factor>() -> Option<usize>;

    fn instance_state_getter<F: Factor>() -> Option<StateGetter<Self, F>> {
        StateGetter::new()
    }

    fn instance_state_getter2<F1: Factor, F2: Factor>() -> Option<StateGetter2<Self, F1, F2>> {
        StateGetter2::new()
    }
}

pub struct StateGetter<T, F> {
    offset: isize,
    _phantom: PhantomData<fn(T) -> F>,
}

impl<T: RuntimeFactors, F: Factor> StateGetter<T, F> {
    fn new() -> Option<Self> {
        Some(Self {
            offset: T::instance_state_offset::<F>()?.try_into().unwrap(),
            _phantom: PhantomData,
        })
    }

    pub fn get_state<'a>(
        &self,
        instance_state: &'a mut T::InstanceState,
    ) -> &'a mut FactorInstanceState<F> {
        let ptr = instance_state as *mut T::InstanceState;
        unsafe { &mut *(ptr.offset(self.offset) as *mut FactorInstanceState<F>) }
    }
}

impl<T: RuntimeFactors, F: Factor> Clone for StateGetter<T, F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: RuntimeFactors, F: Factor> Copy for StateGetter<T, F> {}

pub struct StateGetter2<T, F1, F2> {
    offset1: isize,
    offset2: isize,
    _phantom: PhantomData<fn(T) -> (F1, F2)>,
}

impl<T: RuntimeFactors, F1: Factor, F2: Factor> StateGetter2<T, F1, F2> {
    fn new() -> Option<Self> {
        // Only safe if F1 and F2 are different (and so do not alias)
        if TypeId::of::<F1>() == TypeId::of::<F2>() {
            return None;
        }
        Some(StateGetter2 {
            offset1: T::instance_state_offset::<F1>()?.try_into().unwrap(),
            offset2: T::instance_state_offset::<F2>()?.try_into().unwrap(),
            _phantom: PhantomData,
        })
    }

    pub fn get_states<'a>(
        &self,
        instance_state: &'a mut T::InstanceState,
    ) -> (
        &'a mut FactorInstanceState<F1>,
        &'a mut FactorInstanceState<F2>,
    ) {
        let ptr = instance_state as *mut T::InstanceState;
        unsafe {
            (
                &mut *(ptr.offset(self.offset1) as *mut FactorInstanceState<F1>),
                &mut *(ptr.offset(self.offset2) as *mut FactorInstanceState<F2>),
            )
        }
    }
}

impl<T: RuntimeFactors, F1: Factor, F2: Factor> Clone for StateGetter2<T, F1, F2> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: RuntimeFactors, F1: Factor, F2: Factor> Copy for StateGetter2<T, F1, F2> {}
