//! A Fermyon engine.

#![deny(missing_docs)]

use anyhow::Result;
use std::sync::Arc;
use wasi_common::WasiCtx;
use wasi_experimental_http_wasmtime::HttpCtx;
use wasmtime::{Engine, Instance, InstancePre, Linker, Module, Store};
use wasmtime_wasi::sync::{ambient_authority, Dir, WasiCtxBuilder};

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
        Self {
            env_vars,
            preopen_dirs,
            allowed_http_hosts,
            ..Default::default()
        }
    }

    /// Create a default instance of the configuration instance.
    pub fn default() -> Self {
        // In order for Wasmtime to run WebAssembly components, multi memory
        // and module linking must always be enabled.
        // See https://github.com/bytecodealliance/witx-bindgen/blob/main/crates/wasmlink
        let mut wasmtime_config = wasmtime::Config::default();
        wasmtime_config.wasm_multi_memory(true);
        wasmtime_config.wasm_module_linking(true);

        Self {
            wasmtime_config,
            ..Default::default()
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

/// Builder for the execution context.
#[derive(Default)]
pub struct ExecutionContextBuilder<T: Default> {
    /// Entrypoint of the WebAssembly compnent.
    pub entrypoint_path: String,
    /// Top-level runtime configuration.
    pub config: Config,
    /// Linker used to configure the execution context.
    pub linker: Linker<RuntimeContext<T>>,
    /// Store used to configure the execution context.
    pub store: Store<RuntimeContext<T>>,
    /// Wasmtime engine.
    pub engine: Engine,
}

impl<T: Default> ExecutionContextBuilder<T> {
    /// Create a new instance of the execution builder.
    pub fn new(entrypoint_path: String, config: Config) -> Result<ExecutionContextBuilder<T>> {
        let data = RuntimeContext::default();
        let engine = Engine::new(&config.wasmtime_config)?;
        let store = Store::new(&engine, data);
        let linker = Linker::new(&engine);

        Ok(Self {
            entrypoint_path,
            config,
            linker,
            store,
            engine,
        })
    }

    /// Configure the WASI linker imports for the current execution context.
    pub fn link_wasi<'a>(&'a mut self) -> Result<&'a Self> {
        wasmtime_wasi::add_to_linker(&mut self.linker, |ctx| ctx.wasi.as_mut().unwrap())?;
        Ok(self)
    }

    /// Configure the HTTP linker imports for the current execution context.
    pub fn link_http<'a>(&'a mut self) -> Result<&'a Self> {
        let hosts = &self.config.allowed_http_hosts.clone();
        let http = HttpCtx::new(hosts.clone(), None)?;
        http.add_to_linker(&mut self.linker)?;
        Ok(self)
    }

    /// Build a new instance of the execution context by pre-instantiating the entrypoint module.
    pub fn build(self) -> Result<ExecutionContext<T>> {
        let module = Module::from_file(&self.engine, &self.entrypoint_path)?;
        let pre = self.linker.instantiate_pre(self.store, &module)?;
        Ok(ExecutionContext {
            config: self.config,
            engine: self.engine,
            pre: Arc::new(pre),
        })
    }

    /// Build a new default instance of the execution context by pre-instantiating the entrypoint module.
    pub fn build_default(entrypoint_path: &str, config: Config) -> Result<ExecutionContext<T>> {
        let mut builder = Self::new(entrypoint_path.into(), config)?;
        builder.link_wasi()?;
        builder.link_http()?;

        builder.build()
    }
}

/// The generic execution context.
#[derive(Clone)]
pub struct ExecutionContext<T: Default> {
    /// Top-level runtime configuration.
    pub config: Config,
    /// Wasmtime engine.
    pub engine: Engine,
    /// Pre-initialized WebAssembly instance.
    pub pre: Arc<InstancePre<RuntimeContext<T>>>,
}

impl<T: Default> ExecutionContext<T> {
    /// Prepare an instance with actual data
    pub fn prepare(&self, data: Option<T>) -> Result<(Store<RuntimeContext<T>>, Instance)> {
        let mut store = self.make_store(data)?;
        let instance = self.pre.instantiate(&mut store)?;
        Ok((store, instance))
    }

    fn make_store(&self, data: Option<T>) -> Result<Store<RuntimeContext<T>>> {
        let mut ctx = RuntimeContext::default();
        ctx.data = data;
        let mut wasi_ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .envs(&self.config.env_vars)?;

        for (guest, host) in &self.config.preopen_dirs {
            wasi_ctx =
                wasi_ctx.preopened_dir(Dir::open_ambient_dir(host, ambient_authority())?, guest)?;
        }

        ctx.wasi = Some(wasi_ctx.build());

        let store = Store::new(&self.engine, ctx);
        Ok(store)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use echo::*;
    use std::sync::Arc;

    witx_bindgen_wasmtime::import!("crates/engine/tests/echo.witx");
    const RUST_ENTRYPOINT_PATH: &str = "tests/rust-echo/target/wasm32-wasi/release/echo.wasm";

    #[derive(Clone)]
    pub struct EchoEngine(pub Arc<ExecutionContext<EchoData>>);

    impl EchoEngine {
        pub fn execute(&self, msg: &str) -> Result<String> {
            let (mut store, instance) = self.0.prepare(None)?;
            let e = Echo::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;
            let res = e.echo(&mut store, msg)?;
            Ok(res)
        }
    }

    #[test]
    fn test_rust_echo() {
        let e = ExecutionContextBuilder::build_default(RUST_ENTRYPOINT_PATH, Config::default())
            .unwrap();
        let e = EchoEngine(Arc::new(e));

        assert_eq!(e.execute("Fermyon").unwrap(), "Hello, Fermyon".to_string());
    }
}
