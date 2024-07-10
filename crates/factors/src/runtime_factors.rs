use crate::{factor::FactorInstanceState, App, ConfiguredApp, Factor, Linker, RuntimeConfigSource};

/// Implemented by `#[derive(RuntimeFactors)]`
pub trait RuntimeFactors: Sized + 'static {
    type AppState;
    type InstanceBuilders;
    type InstanceState: GetFactorState + Send + 'static;

    fn init(&mut self, linker: &mut Linker<Self>) -> anyhow::Result<()>;

    fn configure_app(
        &self,
        app: App,
        runtime_config: impl RuntimeConfigSource,
    ) -> anyhow::Result<ConfiguredApp<Self>>;

    fn build_store_data(
        &self,
        configured_app: &ConfiguredApp<Self>,
        component_id: &str,
    ) -> anyhow::Result<Self::InstanceState>;

    fn app_state<F: Factor>(app_state: &Self::AppState) -> Option<&F::AppState>;

    fn instance_builder_mut<F: Factor>(
        builders: &mut Self::InstanceBuilders,
    ) -> Option<Option<&mut F::InstanceBuilder>>;
}

/// Get the state of a particular Factor from the overall InstanceState
///
/// Implemented by `#[derive(RuntimeFactors)]`
pub trait GetFactorState {
    fn get<F: Factor>(&mut self) -> Option<&mut FactorInstanceState<F>>;
}
