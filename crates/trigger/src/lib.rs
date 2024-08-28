pub mod cli;
mod factors;
mod stdio;

use std::future::Future;

pub use factors::TriggerFactors;

use clap::Args;
use spin_core::Linker;
use spin_factors::RuntimeFactors;
use spin_factors_executor::{FactorsExecutorApp, FactorsInstanceBuilder};

pub use spin_app::App;

/// Type alias for a [`FactorsExecutorApp`] specialized to a [`Trigger`].
pub type TriggerApp<T, F> = FactorsExecutorApp<F, <T as Trigger<F>>::InstanceState>;

pub type TriggerInstanceBuilder<'a, T, F> =
    FactorsInstanceBuilder<'a, F, <T as Trigger<F>>::InstanceState>;

pub type Store<T, F> = spin_core::Store<TriggerInstanceState<T, F>>;

type TriggerInstanceState<T, F> = spin_factors_executor::InstanceState<
    <F as RuntimeFactors>::InstanceState,
    <T as Trigger<F>>::InstanceState,
>;

pub trait Trigger<F: RuntimeFactors>: Sized + Send {
    const TYPE: &'static str;

    type CliArgs: Args;
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
