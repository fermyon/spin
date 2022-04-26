use std::sync::Arc;

use crate::{Error, Key, Path, Resolver};

mod wit {
    wit_bindgen_wasmtime::export!("../../wit/ephemeral/spin-config.wit");
}
pub use wit::spin_config::add_to_linker;

/// A component configuration interface implementation.
pub struct ComponentConfig {
    component_root: Path,
    resolver: Arc<Resolver>,
}

impl ComponentConfig {
    pub fn new(component_id: impl Into<String>, resolver: Arc<Resolver>) -> crate::Result<Self> {
        let component_root = Path::new(component_id).or_else(|_| {
            // Temporary mitigation for https://github.com/fermyon/spin/issues/337
            Path::new("invalid.path.issue_337")
        })?;
        Ok(Self {
            component_root,
            resolver,
        })
    }
}

impl wit::spin_config::SpinConfig for ComponentConfig {
    fn get_config(&mut self, key: &str) -> Result<String, wit::spin_config::Error> {
        let key = Key::new(key)?;
        let path = &self.component_root + key;
        Ok(self.resolver.resolve(&path)?)
    }
}

impl From<Error> for wit::spin_config::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::InvalidKey(msg) => Self::InvalidKey(msg),
            Error::InvalidSchema(msg) => Self::InvalidSchema(msg),
            Error::Provider(msg) => Self::Provider(msg.to_string()),
            other => Self::Other(format!("{}", other)),
        }
    }
}
