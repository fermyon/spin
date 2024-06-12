use field_offset::FieldOffset;

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
    fn instance_state_offset<F: Factor>(
    ) -> Option<FieldOffset<Self::InstanceState, FactorInstanceState<F>>>;

    fn instance_state_getter<F: Factor>() -> Option<StateGetter<Self, F>> {
        StateGetter::new()
    }

    fn instance_state_getter2<F1: Factor, F2: Factor>() -> Option<StateGetter2<Self, F1, F2>> {
        StateGetter2::new()
    }
}

pub struct StateGetter<T: RuntimeFactors, F: Factor> {
    offset: FieldOffset<T::InstanceState, FactorInstanceState<F>>,
}

impl<T: RuntimeFactors, F: Factor> StateGetter<T, F> {
    fn new() -> Option<Self> {
        Some(Self {
            offset: T::instance_state_offset::<F>()?,
        })
    }

    pub fn get_state<'a>(
        &self,
        instance_state: &'a mut T::InstanceState,
    ) -> &'a mut FactorInstanceState<F> {
        self.offset.apply_mut(instance_state)
    }
}

impl<T: RuntimeFactors, F: Factor> Clone for StateGetter<T, F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: RuntimeFactors, F: Factor> Copy for StateGetter<T, F> {}

pub struct StateGetter2<T: RuntimeFactors, F1: Factor, F2: Factor> {
    // Invariant: offsets must point at non-overlapping objects
    offset1: FieldOffset<T::InstanceState, FactorInstanceState<F1>>,
    offset2: FieldOffset<T::InstanceState, FactorInstanceState<F2>>,
}

impl<T: RuntimeFactors, F1: Factor, F2: Factor> StateGetter2<T, F1, F2> {
    fn new() -> Option<Self> {
        let offset1 = T::instance_state_offset::<F1>()?;
        let offset2 = T::instance_state_offset::<F2>()?;
        // Make sure the two states don't point to the same field
        if offset1.get_byte_offset() == offset2.get_byte_offset() {
            return None;
        }
        Some(StateGetter2 { offset1, offset2 })
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
                &mut *(self.offset1.apply_ptr_mut(ptr) as *mut FactorInstanceState<F1>),
                &mut *(self.offset2.apply_ptr_mut(ptr) as *mut FactorInstanceState<F2>),
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

// TODO: This seems fine, but then again I don't understand why `FieldOffset`'s
// own `Sync`ness depends on `U`...
unsafe impl<T: RuntimeFactors, F1: Factor, F2: Factor> Sync for StateGetter2<T, F1, F2> {}
