use crate::Factor;

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
