use super::Context;
use anyhow::ensure;
use std::{collections::HashMap, error, fmt};
use wasmtime::{InstancePre, Store};

pub use spin_config::add_to_linker;

wit_bindgen_wasmtime::export!("../../wit/ephemeral/spin-config.wit");

impl fmt::Display for spin_config::Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Provider(provider_err) => write!(f, "provider error: {}", provider_err),
            Self::InvalidKey(invalid_key) => write!(f, "invalid key: {}", invalid_key),
            Self::InvalidSchema(invalid_schema) => {
                write!(f, "invalid schema: {}", invalid_schema)
            }
            Self::Other(other) => write!(f, "other: {}", other),
        }
    }
}

impl error::Error for spin_config::Error {}

#[derive(Default)]
pub(super) struct SpinConfig {
    map: HashMap<String, String>,
}

impl spin_config::SpinConfig for SpinConfig {
    fn get_config(&mut self, key: &str) -> Result<String, spin_config::Error> {
        self.map
            .remove(key)
            .ok_or_else(|| spin_config::Error::InvalidKey(key.to_owned()))
    }
}

pub(super) fn test(store: &mut Store<Context>, pre: &InstancePre<Context>) -> Result<(), String> {
    store
        .data_mut()
        .spin_config
        .map
        .insert("foo".into(), "bar".into());

    super::run_command(store, pre, &["config", "foo"], |store| {
        ensure!(
            store.data().spin_config.map.is_empty(),
            "expected module to call `spin-config::get-config` exactly once"
        );

        Ok(())
    })
}
