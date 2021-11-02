//! A Fermyon engine.

#![deny(missing_docs)]

use std::sync::Arc;
use wasi_common::WasiCtx;
use wasmtime::{Engine, InstancePre};

/// Engine configuration.
#[derive(Clone, Default)]
pub struct Config {
    /// Environment variables to set inside the WebAssembly module.
    pub env_vars: Vec<(String, String)>,
    /// Preopened directories to map inside the WebAssembly module.
    pub preopen_dirs: Vec<(String, String)>,
    /// Optional list of HTTP hosts WebAssembly modules are allowed to connect to.
    pub allowed_http_hosts: Option<Vec<String>>,
    /// Wasmtime engine configuration.
    pub wasmtime_config: wasmtime::Config,
}

impl Config {
    /// Create a new configuration instance.
    pub fn new(
        env_vars: Vec<(String, String)>,
        preopen_dirs: Vec<(String, String)>,
        allowed_http_hosts: Option<Vec<String>>,
    ) -> Self {
        let mut wasmtime_config = wasmtime::Config::default();
        wasmtime_config.wasm_multi_memory(true);
        wasmtime_config.wasm_module_linking(true);

        Self {
            env_vars,
            preopen_dirs,
            allowed_http_hosts,
            wasmtime_config,
        }
    }
}

/// Top-level runtime context data to be passed to a WebAssembly module.
#[derive(Default)]
pub struct RuntimeContext<T> {
    /// WASI context data.
    pub wasi: Option<WasiCtx>,
    /// Generic runtime data that can be configured by specific engines.
    pub data: Option<T>,
}

/// The generic execution context.
#[derive(Clone)]
pub struct ExecutionContext<T: Default> {
    /// Entrypoint of the WebAssembly compnent.
    pub entrypoint_path: String,
    /// Top-level runtime configuration.
    pub config: Config,
    /// Pre-initialized WebAssembly instance.
    pub pre: Arc<InstancePre<RuntimeContext<T>>>,
    /// Wasmtime engine.
    pub engine: Engine,
}
