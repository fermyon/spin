mod factor;
mod prepare;
mod runtime_config;
mod runtime_factors;

pub use anyhow;
pub use wasmtime;

pub use spin_factors_derive::RuntimeFactors;

pub use crate::{
    factor::{ConfigureAppContext, ConfiguredApp, Factor, FactorInstanceState, InitContext},
    prepare::{FactorInstanceBuilder, InstanceBuilders, PrepareContext, SelfInstanceBuilder},
    runtime_config::{FactorRuntimeConfig, RuntimeConfigSource},
    runtime_factors::{GetFactorState, RuntimeFactors},
};

pub type Linker<T> = wasmtime::component::Linker<<T as RuntimeFactors>::InstanceState>;

// Temporary wrappers while refactoring
pub type App = spin_app::App<'static, spin_app::InertLoader>;
pub type AppComponent<'a> = spin_app::AppComponent<'a, spin_app::InertLoader>;

// TODO: Add a real Error type
pub type Result<T> = wasmtime::Result<T>;

#[doc(hidden)]
pub mod __internal {
    pub use crate::runtime_config::RuntimeConfigTracker;
}
