use std::{
    fmt::{Debug, Display},
    path::Path,
    sync::{Arc, RwLock},
};

use anyhow::Context;
use liquid_core::{Filter, ParseFilter, Runtime, ValueView};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

wit_bindgen_wasmtime::import!({paths: ["./wit/custom-filter.wit"]});

struct CustomFilterContext {
    wasi: WasiCtx,
    data: custom_filter::CustomFilterData,
}

impl CustomFilterContext {
    fn new() -> Self {
        Self {
            wasi: WasiCtxBuilder::new().build(),
            data: custom_filter::CustomFilterData {},
        }
    }
}

#[derive(Clone)]
pub(crate) struct CustomFilterParser {
    name: String,
    wasm_store: Arc<RwLock<Store<CustomFilterContext>>>,
    exec: Arc<custom_filter::CustomFilter<CustomFilterContext>>,
}

impl CustomFilterParser {
    pub(crate) fn load(name: &str, wasm_path: &Path) -> anyhow::Result<Self> {
        let wasm = std::fs::read(wasm_path).with_context(|| {
            format!("Failed loading custom filter from {}", wasm_path.display())
        })?;

        let ctx = CustomFilterContext::new();
        let engine = Engine::default();
        let mut store = Store::new(&engine, ctx);
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |ctx: &mut CustomFilterContext| &mut ctx.wasi)
            .with_context(|| format!("Setting up WASI for custom filter {}", name))?;
        let module = Module::new(&engine, wasm)
            .with_context(|| format!("Creating Wasm module for custom filter {}", name))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .with_context(|| format!("Instantiating Wasm module for custom filter {}", name))?;
        let filter_exec =
            custom_filter::CustomFilter::new(&mut store, &instance, |ctx| &mut ctx.data)
                .with_context(|| format!("Loading Wasm executor for custom filer {}", name))?;

        Ok(Self {
            name: name.to_owned(),
            wasm_store: Arc::new(RwLock::new(store)),
            exec: Arc::new(filter_exec),
        })
    }
}

impl Debug for CustomFilterParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomFilterParser")
            .field("name", &self.name)
            .finish()
    }
}

impl ParseFilter for CustomFilterParser {
    fn parse(
        &self,
        _arguments: liquid_core::parser::FilterArguments,
    ) -> liquid_core::Result<Box<dyn Filter>> {
        Ok(Box::new(CustomFilter {
            name: self.name.to_owned(),
            wasm_store: self.wasm_store.clone(),
            exec: self.exec.clone(),
        }))
    }

    fn reflection(&self) -> &dyn liquid_core::FilterReflection {
        self
    }
}

const EMPTY: [liquid_core::parser::ParameterReflection; 0] = [];

impl liquid_core::FilterReflection for CustomFilterParser {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        ""
    }

    fn positional_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &EMPTY
    }

    fn keyword_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &EMPTY
    }
}

struct CustomFilter {
    name: String,
    wasm_store: Arc<RwLock<Store<CustomFilterContext>>>,
    exec: Arc<custom_filter::CustomFilter<CustomFilterContext>>,
}

impl Debug for CustomFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomFilter")
            .field("name", &self.name)
            .finish()
    }
}

impl Display for CustomFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl Filter for CustomFilter {
    fn evaluate(
        &self,
        input: &dyn ValueView,
        _runtime: &dyn Runtime,
    ) -> Result<liquid::model::Value, liquid_core::error::Error> {
        let mut store = self
            .wasm_store
            .write()
            .map_err(|e| liquid_err(format!("Failed to get custom filter Wasm store: {}", e)))?;
        let input_str = self.liquid_value_as_string(input)?;
        match self.exec.exec(&mut *store, &input_str) {
            Ok(Ok(text)) => Ok(to_liquid_value(text)),
            Ok(Err(s)) => Err(liquid_err(s)),
            Err(trap) => Err(liquid_err(format!("{:?}", trap))),
        }
    }
}

impl CustomFilter {
    fn liquid_value_as_string(&self, input: &dyn ValueView) -> Result<String, liquid::Error> {
        let str = input.as_scalar().map(|s| s.into_cow_str()).ok_or_else(|| {
            liquid_err(format!(
                "Filter '{}': no input or input is not a string",
                self.name
            ))
        })?;
        Ok(str.to_string())
    }
}

fn to_liquid_value(value: String) -> liquid::model::Value {
    liquid::model::Value::Scalar(liquid::model::Scalar::from(value))
}

fn liquid_err(text: String) -> liquid_core::error::Error {
    liquid_core::error::Error::with_msg(text)
}
