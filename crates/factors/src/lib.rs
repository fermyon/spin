mod factor;
mod instance_preparer;
mod runtime_config;
mod spin_factors;

pub use anyhow;
pub use wasmtime;

pub use spin_factors_derive::SpinFactors;

pub use crate::{
    factor::{ConfigureAppContext, ConfiguredApp, Factor, InitContext},
    instance_preparer::{FactorInstancePreparer, InstancePreparers, PrepareContext},
    runtime_config::{RuntimeConfig, RuntimeConfigSource},
    spin_factors::SpinFactors,
};

pub type Linker<Factors> = wasmtime::component::Linker<<Factors as SpinFactors>::InstanceState>;
pub type ModuleLinker<Factors> = wasmtime::Linker<<Factors as SpinFactors>::InstanceState>;

// Temporary wrappers while refactoring
pub type App = spin_app::App<'static, spin_app::InertLoader>;
pub type AppComponent<'a> = spin_app::AppComponent<'a, spin_app::InertLoader>;

// TODO: Add a real Error type
pub type Result<T> = wasmtime::Result<T>;

#[doc(hidden)]
pub mod __internal {
    pub use crate::runtime_config::RuntimeConfigTracker;
}
