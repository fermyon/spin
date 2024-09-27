pub mod cli;
pub mod loader;

use std::future::Future;

use clap::Args;
pub use spin_core::Linker;
pub use spin_factors::RuntimeFactors;
use spin_factors_executor::{FactorsExecutorApp, FactorsInstanceBuilder};

pub use anyhow;
pub use clap::Parser;
pub use spin_app::App;

/// Type alias for a [`spin_factors_executor::FactorsExecutorApp`] specialized to a [`Trigger`].
pub type TriggerApp<T, F> = FactorsExecutorApp<F, <T as Trigger<F>>::InstanceState>;

/// Type alias for a [`spin_factors_executor::FactorsInstanceBuilder`] specialized to a [`Trigger`].
pub type TriggerInstanceBuilder<'a, T, F> =
    FactorsInstanceBuilder<'a, F, <T as Trigger<F>>::InstanceState>;

/// Type alias for a [`spin_core::Store`] specialized to a [`Trigger`].
pub type Store<T, F> = spin_core::Store<TriggerInstanceState<T, F>>;

/// Type alias for [`spin_factors_executor::InstanceState`] specialized to a [`Trigger`].
pub type TriggerInstanceState<T, F> = spin_factors_executor::InstanceState<
    <F as RuntimeFactors>::InstanceState,
    <T as Trigger<F>>::InstanceState,
>;

/// A trigger for a Spin runtime.
pub trait Trigger<F: RuntimeFactors>: Sized + Send {
    /// A unique identifier for this trigger.
    const TYPE: &'static str;

    /// The specific CLI arguments for this trigger.
    type CliArgs: Args;

    /// The instance state for this trigger.
    type InstanceState: Send + 'static;

    /// Constructs a new trigger.
    fn new(cli_args: Self::CliArgs, app: &App) -> anyhow::Result<Self>;

    /// Update the [`spin_core::Config`] for this trigger.
    ///
    /// !!!Warning!!! This is unsupported; many configurations are likely to
    /// cause errors or unexpected behavior, especially in future versions.
    #[doc(hidden)]
    fn update_core_config(&mut self, config: &mut spin_core::Config) -> anyhow::Result<()> {
        let _ = config;
        Ok(())
    }

    /// Update the [`Linker`] for this trigger.
    fn add_to_linker(
        &mut self,
        linker: &mut Linker<TriggerInstanceState<Self, F>>,
    ) -> anyhow::Result<()> {
        let _ = linker;
        Ok(())
    }

    /// Run this trigger.
    fn run(
        self,
        trigger_app: TriggerApp<Self, F>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Returns a list of host requirements supported by this trigger specifically.
    ///
    /// See [`App::ensure_needs_only`].
    fn supported_host_requirements() -> Vec<&'static str> {
        Vec::new()
    }
}
