mod factor;
mod prepare;
mod runtime_config;
mod runtime_factors;

pub use anyhow;
pub use serde;
pub use wasmtime;

pub use spin_factors_derive::RuntimeFactors;

pub use crate::{
    factor::{ConfigureAppContext, ConfiguredApp, Factor, FactorInstanceState, InitContext},
    prepare::{FactorInstanceBuilder, InstanceBuilders, PrepareContext, SelfInstanceBuilder},
    runtime_config::{FactorRuntimeConfig, RuntimeConfigSource},
    runtime_factors::{GetFactorState, RuntimeFactors},
};

/// A [`wasmtime::component::Linker`] used for a [`RuntimeFactors`] collection.
pub type Linker<T> = wasmtime::component::Linker<<T as RuntimeFactors>::InstanceState>;

// Temporary wrappers while refactoring
pub type App = spin_app::App<'static, spin_app::InertLoader>;
pub type AppComponent<'a> = spin_app::AppComponent<'a, spin_app::InertLoader>;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("factor dependency ordering error: {0}")]
    DependencyOrderingError(String),
    #[error("no such factor: {0}")]
    NoSuchFactor(&'static str),
    #[error("{factor} requested already-consumed key {key:?}")]
    RuntimeConfigReusedKey { factor: &'static str, key: String },
    #[error("runtime config error: {0}")]
    RuntimeConfigSource(#[source] anyhow::Error),
    #[error("unused runtime config key(s): {}", keys.join(", "))]
    RuntimeConfigUnusedKeys { keys: Vec<String> },
    #[error("{factor} {method} failed: {source}")]
    RuntimeFactorError {
        factor: &'static str,
        method: &'static str,
        source: anyhow::Error,
    },
    #[error("unknown component: {0}")]
    UnknownComponent(String),
}

impl Error {
    fn no_such_factor<T: Factor>() -> Self {
        Self::NoSuchFactor(std::any::type_name::<T>())
    }

    fn runtime_config_reused_key<T: Factor>(key: impl Into<String>) -> Self {
        Self::RuntimeConfigReusedKey {
            factor: std::any::type_name::<T>(),
            key: key.into(),
        }
    }
}

#[doc(hidden)]
pub mod __internal {
    pub use crate::runtime_config::RuntimeConfigTracker;

    pub fn runtime_factor_error<T: crate::Factor>(
        method: &'static str,
        source: anyhow::Error,
    ) -> crate::Error {
        crate::Error::RuntimeFactorError {
            factor: std::any::type_name::<T>(),
            method,
            source,
        }
    }
}
