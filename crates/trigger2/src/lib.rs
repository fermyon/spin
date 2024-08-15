use std::future::Future;

use clap::Args;
use spin_app::App;
use spin_core::Linker;
use spin_factors::RuntimeFactors;
use spin_factors_executor::{FactorsExecutorApp, FactorsInstanceBuilder};

pub mod cli;
mod factors;
mod stdio;

/// Type alias for a [`FactorsExecutorApp`] specialized to a [`Trigger`].
pub type TriggerApp<T> =
    FactorsExecutorApp<<T as Trigger>::RuntimeFactors, <T as Trigger>::InstanceState>;

pub type TriggerInstanceBuilder<'a, T> =
    FactorsInstanceBuilder<'a, <T as Trigger>::RuntimeFactors, <T as Trigger>::InstanceState>;

pub type Store<T> = spin_core::Store<TriggerInstanceState<T>>;

type TriggerInstanceState<T> = spin_factors_executor::InstanceState<
    <<T as Trigger>::RuntimeFactors as RuntimeFactors>::InstanceState,
    <T as Trigger>::InstanceState,
>;

pub trait Trigger: Sized + Send {
    const TYPE: &'static str;

    type CliArgs: Args;
    type InstanceState: Send + 'static;
    type RuntimeFactors: RuntimeFactors;

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
        linker: &mut Linker<TriggerInstanceState<Self>>,
    ) -> anyhow::Result<()> {
        let _ = linker;
        Ok(())
    }

    /// Run this trigger.
    fn run(
        self,
        configured_app: TriggerApp<Self>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Returns a list of host requirements supported by this trigger specifically.
    ///
    /// See [`App::ensure_needs_only`].
    fn supported_host_requirements() -> Vec<&'static str> {
        Vec::new()
    }

    /// Create factors
    fn create_factors(&self, working_dir: &str) -> Self::RuntimeFactors;
}
