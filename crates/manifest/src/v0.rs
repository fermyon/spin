use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_value::Value;
use serde_with::{As, TryFromInto};

use crate::{v1, ManifestVersion};

/// Manifest represents a (deprecated) "V0" application manifest.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(with = "As::<TryFromInto<String>>")]
    spin_version: ManifestVersion<1>,
    name: String,
    version: String,
    description: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    trigger: ApplicationTrigger,
    // TODO(lann): What is this for?
    namespace: Option<String>,
    #[serde(default)]
    config: spin_config::Tree,
    component: Vec<ComponentManifest>,
}

impl TryInto<v1::Manifest> for Manifest {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<v1::Manifest, Self::Error> {
        let trigger_type = match self.trigger {
            ApplicationTrigger::Http { .. } => v1::TriggerType::new("http"),
            ApplicationTrigger::Redis { .. } => v1::TriggerType::new("redis"),
        };

        let mut triggers: HashMap<v1::TriggerType, Vec<v1::TriggerConfig>> = HashMap::new();
        let mut components = vec![];
        for component in self.component {
            let (component, trigger) = component.try_into()?;
            components.push(component);
            if let Some(config) = trigger {
                match triggers.get_mut(&trigger_type) {
                    Some(triggers) => triggers.push(config),
                    None => {
                        triggers.insert(trigger_type.clone(), vec![config]);
                    }
                };
            }
        }

        Ok(v1::Manifest {
            spin_manifest_version: Default::default(),
            application: v1::ApplicationConfig {
                name: self.name,
                version: self.version,
                description: self.description,
                authors: self.authors,
                trigger_configs: self.trigger.try_into()?,
            },
            variables: self.config,
            triggers,
            components,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", tag = "type")]
pub enum ApplicationTrigger {
    Http { base: String },
    Redis { address: String },
}

impl TryFrom<ApplicationTrigger> for HashMap<v1::TriggerType, v1::TriggerConfig> {
    type Error = anyhow::Error;

    fn try_from(
        value: ApplicationTrigger,
    ) -> anyhow::Result<HashMap<v1::TriggerType, v1::TriggerConfig>> {
        let (key, values): (&str, v1::TriggerConfig) = match value {
            ApplicationTrigger::Http { base } => {
                ("http", [("base".to_string(), Value::String(base))].into())
            }
            ApplicationTrigger::Redis { address } => (
                "redis",
                [("address".to_string(), Value::String(address))].into(),
            ),
        };
        Ok([(key.to_string().try_into()?, values)].into())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ComponentManifest {
    source: ModuleSource,
    id: String,
    description: Option<String>,
    #[serde(default)]
    environment: HashMap<String, String>,
    #[serde(default)]
    files: Vec<FileMount>,
    #[serde(default)]
    allowed_http_hosts: Vec<String>,
    trigger: Option<TriggerConfig>,
    #[serde(default)]
    config: HashMap<String, String>,
    build: Option<BuildConfig>,
}

impl TryFrom<ComponentManifest> for (v1::ComponentManifest, Option<v1::TriggerConfig>) {
    type Error = anyhow::Error;

    fn try_from(component: ComponentManifest) -> Result<Self, Self::Error> {
        let v1_component = v1::ComponentManifest {
            id: component.id.try_into()?,
            description: component.description,
            source: component.source.try_into()?,
            environment: component.environment,
            files: component.files.into_iter().map(Into::into).collect(),
            config: component.config,
            build: component.build.map(Into::into),
        };

        let v1_trigger = component
            .trigger
            .map(serde_value::to_value)
            .transpose()?
            .map(|value| value.deserialize_into())
            .transpose()?
            .map(|mut trigger: HashMap<_, _>| {
                trigger.insert(
                    "component".to_string(),
                    Value::String(v1_component.id.as_ref().to_string()),
                );
                trigger
            });

        Ok((v1_component, v1_trigger))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
enum ModuleSource {
    FileReference(PathBuf),
    Bindle { reference: String, parcel: String },
}

impl From<ModuleSource> for v1::ComponentSource {
    fn from(source: ModuleSource) -> Self {
        match source {
            ModuleSource::FileReference(path) => v1::ComponentSource::Local(path),
            ModuleSource::Bindle { reference, parcel } => v1::ComponentSource::Bindle {
                bindle: reference,
                parcel,
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
enum FileMount {
    Pattern(String),
    Placement {
        source: PathBuf,
        destination: PathBuf,
    },
}

impl From<FileMount> for v1::FileMapping {
    fn from(value: FileMount) -> Self {
        match value {
            FileMount::Pattern(pattern) => Self::Pattern(pattern),
            FileMount::Placement {
                source,
                destination,
            } => Self::Placement {
                source,
                destination,
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
enum TriggerConfig {
    Http {
        route: String,
        executor: Option<HttpExecutor>,
    },
    Redis {
        channel: String,
        executor: Option<String>,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", tag = "type")]
enum HttpExecutor {
    Spin,
    Wagi { entrypoint: String, argv: String },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildConfig {
    command: String,
    workdir: Option<PathBuf>,
}

impl From<BuildConfig> for v1::ComponentBuildConfig {
    fn from(config: BuildConfig) -> Self {
        v1::ComponentBuildConfig {
            command: config.command,
            workdir: config.workdir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_v1() {
        let manifest: Manifest = toml::toml! {
            spin_version = "1"
            name = "v0-app"
            version = "1.2.3"
            description = "A V0 manifest"
            authors = ["Las Autoras"]
            trigger = { type = "http", base = "/base" }
            namespace = "fermyon"

            [config]
            key = { default = "value" }

            [[component]]
            id = "my-component"
            description = "My Component"
            files = ["file.txt", { source = "src", destination = "dst" }]
            [component.source]
            parcel = "parcel"
            reference = "bindle reference"
            [component.trigger]
            executor = { type = "wagi", entrypoint = "start", argv = "serve"}
            route = "/my"
            [component.environment]
            KEY = "value"

            [[component]]
            id = "other-component"
            source = "other.wasm"
            [component.trigger]
            route = "/other"

        }
        .try_into()
        .unwrap();

        let v1: v1::Manifest = manifest.try_into().expect("conversion failed");
        let http = v1::TriggerType::new("http");

        let app = &v1.application;
        assert_eq!(app.name, "v0-app");
        assert_eq!(app.version, "1.2.3");
        assert_eq!(app.description.as_ref().unwrap(), "A V0 manifest");
        assert_eq!(app.authors[0], "Las Autoras");
        assert_eq!(
            app.trigger_configs[&http]["base"],
            Value::String("/base".to_string()),
        );

        let component = &v1.components[0];
        assert_eq!(component.id, v1::ComponentId::new("my-component"));
        assert_eq!(component.description.as_deref(), Some("My Component"));
        assert!(
            matches!(component.files[0], v1::FileMapping::Pattern(ref pat)
                if pat == "file.txt")
        );
        assert!(
            matches!(component.files[1], v1::FileMapping::Placement { ref source, ref destination }
                if source.as_os_str() == "src" && destination.as_os_str() == "dst")
        );
        assert!(
            matches!(component.source, v1::ComponentSource::Bindle { ref bindle, ref parcel }
                if bindle == "bindle reference" && parcel == "parcel")
        );
        assert_eq!(component.environment["KEY"], "value");

        let trigger = &v1.triggers[&http][0];
        assert_eq!(
            trigger["component"],
            Value::String("my-component".to_string())
        );
        assert_eq!(trigger["route"], Value::String("/my".to_string()));

        let executor_value = match &trigger["executor"] {
            Value::Option(Some(executor)) => executor.as_ref(),
            wrong => panic!("wrong type: {:?}", wrong),
        };
        match executor_value {
            Value::Map(ref executor) => {
                for (k, v) in [("type", "wagi"), ("entrypoint", "start"), ("argv", "serve")] {
                    assert_eq!(
                        executor[&Value::String(k.to_string())],
                        Value::String(v.to_string())
                    );
                }
            }
            wrong => panic!("wrong type: {:?}", wrong),
        }

        assert!(
            matches!(v1.components[1].source, v1::ComponentSource::Local(ref path)
                if path.as_os_str() == "other.wasm")
        );
        assert_eq!(
            v1.triggers[&http][1]["component"],
            Value::String("other-component".to_string())
        );
    }
}
