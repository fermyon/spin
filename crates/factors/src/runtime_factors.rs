use crate::Factor;

// TODO(lann): Most of the unsafe shenanigans here probably aren't worth it;
// consider replacing with e.g. `Any::downcast`.

/// Implemented by `#[derive(RuntimeFactors)]`
pub trait RuntimeFactors: Sized {
    type AppState;
    type InstanceBuilders;
    type InstanceState: Send + 'static;

    fn app_state<F: Factor>(app_state: &Self::AppState) -> Option<&F::AppState>;

    fn instance_builder_mut<F: Factor>(
        builders: &mut Self::InstanceBuilders,
    ) -> Option<Option<&mut F::InstanceBuilder>>;
}
