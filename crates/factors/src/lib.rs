mod factor;
mod prepare;
mod runtime_config;
mod spin_factors;

pub use anyhow;
pub use wasmtime;

pub use spin_factors_derive::RuntimeFactors;

pub use crate::{
    factor::{ConfigureAppContext, ConfiguredApp, Factor, InitContext},
    prepare::{InstancePreparers, PrepareContext},
    runtime_config::{RuntimeConfig, RuntimeConfigSource},
    spin_factors::RuntimeFactors,
};

pub type Linker<T> = wasmtime::component::Linker<<T as RuntimeFactors>::InstanceState>;
pub type ModuleLinker<T> = wasmtime::Linker<<T as RuntimeFactors>::InstanceState>;

// Temporary wrappers while refactoring
pub type App = spin_app::App<'static, spin_app::InertLoader>;
pub type AppComponent<'a> = spin_app::AppComponent<'a, spin_app::InertLoader>;

// TODO: Add a real Error type
pub type Result<T> = wasmtime::Result<T>;

#[doc(hidden)]
pub mod __internal {
    pub use crate::runtime_config::RuntimeConfigTracker;
}
