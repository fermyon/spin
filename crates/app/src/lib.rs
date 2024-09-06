//! Spin internal application interfaces
//!
//! This crate contains interfaces to Spin application configuration to be used
//! by crates that implement Spin execution environments: trigger executors and
//! host components, in particular.

#![deny(missing_docs)]

use serde::Deserialize;
use serde_json::Value;
use spin_locked_app::MetadataExt;

use locked::{ContentPath, LockedApp, LockedComponent, LockedComponentSource, LockedTrigger};

pub use spin_locked_app::locked;
pub use spin_locked_app::values;
pub use spin_locked_app::{Error, MetadataKey, Result};

pub use locked::Variable;

/// MetadataKey for extracting the application name.
pub const APP_NAME_KEY: MetadataKey = MetadataKey::new("name");
/// MetadataKey for extracting the application version.
pub const APP_VERSION_KEY: MetadataKey = MetadataKey::new("version");
/// MetadataKey for extracting the application description.
pub const APP_DESCRIPTION_KEY: MetadataKey = MetadataKey::new("description");
/// MetadataKey for extracting the OCI image digest.
pub const OCI_IMAGE_DIGEST_KEY: MetadataKey = MetadataKey::new("oci_image_digest");

/// An `App` holds loaded configuration for a Spin application.
#[derive(Debug, Clone)]
pub struct App {
    id: String,
    locked: LockedApp,
}

impl App {
    /// Returns a new app for the given runtime-specific identifier and locked
    /// app.
    pub fn new(id: impl Into<String>, locked: LockedApp) -> Self {
        Self {
            id: id.into(),
            locked,
        }
    }

    /// Returns a runtime-specific identifier for this app.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Deserializes typed metadata for this app.
    ///
    /// Returns `Ok(None)` if there is no metadata for the given `key` and an
    /// `Err` only if there _is_ a value for the `key` but the typed
    /// deserialization failed.
    pub fn get_metadata<'this, T: Deserialize<'this>>(
        &'this self,
        key: MetadataKey<T>,
    ) -> Result<Option<T>> {
        self.locked.get_metadata(key)
    }

    /// Deserializes typed metadata for this app.
    ///
    /// Like [`App::get_metadata`], but returns an error if there is
    /// no metadata for the given `key`.
    pub fn require_metadata<'this, T: Deserialize<'this>>(
        &'this self,
        key: MetadataKey<T>,
    ) -> Result<T> {
        self.locked.require_metadata(key)
    }

    /// Returns an iterator of custom config [`Variable`]s defined for this app.
    pub fn variables(&self) -> impl Iterator<Item = (&String, &Variable)> {
        self.locked.variables.iter()
    }

    /// Returns an iterator of [`AppComponent`]s defined for this app.
    pub fn components(&self) -> impl Iterator<Item = AppComponent<'_>> {
        self.locked
            .components
            .iter()
            .map(|locked| AppComponent { app: self, locked })
    }

    /// Returns the [`AppComponent`] with the given `component_id`, or `None`
    /// if it doesn't exist.
    pub fn get_component(&self, component_id: &str) -> Option<AppComponent<'_>> {
        self.components()
            .find(|component| component.locked.id == component_id)
    }

    /// Returns an iterator of [`AppTrigger`]s defined for this app.
    pub fn triggers(&self) -> impl Iterator<Item = AppTrigger<'_>> + '_ {
        self.locked
            .triggers
            .iter()
            .map(|locked| AppTrigger { app: self, locked })
    }

    /// Returns the trigger metadata for a specific trigger type.
    pub fn get_trigger_metadata<'this, T: Deserialize<'this>>(
        &'this self,
        trigger_type: &str,
    ) -> Result<Option<T>> {
        let Some(value) = self.get_trigger_metadata_value(trigger_type) else {
            return Ok(None);
        };
        let metadata = T::deserialize(value).map_err(|err| {
            Error::MetadataError(format!(
                "invalid metadata value for {trigger_type:?}: {err:?}"
            ))
        })?;
        Ok(Some(metadata))
    }

    fn get_trigger_metadata_value(&self, trigger_type: &str) -> Option<Value> {
        if let Some(trigger_configs) = self.locked.metadata.get("triggers") {
            // New-style: `{"triggers": {"<type>": {...}}}`
            trigger_configs.get(trigger_type).cloned()
        } else if self.locked.metadata["trigger"]["type"] == trigger_type {
            // Old-style: `{"trigger": {"type": "<type>", ...}}`
            let mut meta = self.locked.metadata["trigger"].clone();
            meta.as_object_mut().unwrap().remove("type");
            Some(meta)
        } else {
            None
        }
    }

    /// Returns an iterator of [`AppTrigger`]s defined for this app with
    /// the given `trigger_type`.
    pub fn triggers_with_type<'a>(
        &'a self,
        trigger_type: &'a str,
    ) -> impl Iterator<Item = AppTrigger> {
        self.triggers()
            .filter(move |trigger| trigger.locked.trigger_type == trigger_type)
    }

    /// Returns an iterator of trigger IDs and deserialized trigger configs for
    /// the given `trigger_type`.
    pub fn trigger_configs<'a, T: Deserialize<'a>>(
        &'a self,
        trigger_type: &'a str,
    ) -> Result<impl IntoIterator<Item = (&'a str, T)>> {
        self.triggers_with_type(trigger_type)
            .map(|trigger| {
                let config = trigger.typed_config::<T>()?;
                Ok((trigger.id(), config))
            })
            .collect::<Result<Vec<_>>>()
    }

    /// Checks that the application does not have any host requirements
    /// outside the supported set. The error case returns a comma-separated
    /// list of unmet requirements.
    pub fn ensure_needs_only(&self, supported: &[&str]) -> std::result::Result<(), String> {
        self.locked.ensure_needs_only(supported)
    }
}

/// An `AppComponent` holds configuration for a Spin application component.
pub struct AppComponent<'a> {
    /// The app this component belongs to.
    pub app: &'a App,
    /// The locked component.
    pub locked: &'a LockedComponent,
}

impl<'a> AppComponent<'a> {
    /// Returns this component's app-unique ID.
    pub fn id(&self) -> &str {
        &self.locked.id
    }

    /// Returns this component's Wasm component or module source.
    pub fn source(&self) -> &LockedComponentSource {
        &self.locked.source
    }

    /// Returns an iterator of environment variable (key, value) pairs.
    pub fn environment(&self) -> impl IntoIterator<Item = (&str, &str)> {
        self.locked
            .env
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Returns an iterator of [`ContentPath`]s for this component's configured
    /// "directory mounts".
    pub fn files(&self) -> std::slice::Iter<ContentPath> {
        self.locked.files.iter()
    }

    /// Deserializes typed metadata for this component.
    ///
    /// Returns `Ok(None)` if there is no metadata for the given `key` and an
    /// `Err` only if there _is_ a value for the `key` but the typed
    /// deserialization failed.
    pub fn get_metadata<T: Deserialize<'a>>(&self, key: MetadataKey<T>) -> Result<Option<T>> {
        self.locked.metadata.get_typed(key)
    }

    /// Deserializes typed metadata for this component.
    ///
    /// Like [`AppComponent::get_metadata`], but returns an error if there is
    /// no metadata for the given `key`.
    pub fn require_metadata<'this, T: Deserialize<'this>>(
        &'this self,
        key: MetadataKey<T>,
    ) -> Result<T> {
        self.locked.metadata.require_typed(key)
    }

    /// Returns an iterator of custom config values for this component.
    pub fn config(&self) -> impl Iterator<Item = (&String, &String)> {
        self.locked.config.iter()
    }
}

/// An `AppTrigger` holds configuration for a Spin application trigger.
pub struct AppTrigger<'a> {
    /// The app this trigger belongs to.
    pub app: &'a App,
    locked: &'a LockedTrigger,
}

impl<'a> AppTrigger<'a> {
    /// Returns this trigger's app-unique ID.
    pub fn id(&self) -> &'a str {
        &self.locked.id
    }

    /// Returns the Trigger's type.
    pub fn trigger_type(&self) -> &'a str {
        &self.locked.trigger_type
    }

    /// Deserializes this trigger's configuration into a typed value.
    pub fn typed_config<Config: Deserialize<'a>>(&self) -> Result<Config> {
        Ok(Config::deserialize(&self.locked.trigger_config)?)
    }

    /// Returns a reference to the [`AppComponent`] configured for this trigger.
    ///
    /// This is a convenience wrapper that looks up the component based on the
    /// 'component' metadata value which is conventionally a component ID.
    pub fn component(&self) -> Result<AppComponent<'a>> {
        let id = &self.locked.id;
        let common_config: CommonTriggerConfig = self.typed_config()?;
        let component_id = common_config.component.ok_or_else(|| {
            Error::MetadataError(format!("trigger {id:?} missing 'component' config field"))
        })?;
        self.app.get_component(&component_id).ok_or_else(|| {
            Error::MetadataError(format!(
                "missing component {component_id:?} configured for trigger {id:?}"
            ))
        })
    }
}

#[derive(Deserialize)]
struct CommonTriggerConfig {
    component: Option<String>,
}
