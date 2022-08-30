use std::path::PathBuf;

use anyhow::{bail, ensure, Context, Result};
use spin_app::{
    locked::{
        self, ContentPath, ContentRef, LockedApp, LockedComponent, LockedComponentSource,
        LockedTrigger,
    },
    values::{ValuesMap, ValuesMapBuilder},
};
use spin_manifest::{
    Application, ApplicationInformation, ApplicationTrigger, CoreComponent, HttpConfig,
    HttpTriggerConfiguration, RedisConfig, RedisTriggerConfiguration, TriggerConfig,
};

const WASM_CONTENT_TYPE: &str = "application/wasm";

/// Construct a LockedApp from the given Application. Any buffered component
/// sources will be written to the given `working_dir`.
pub fn build_locked_app(app: Application, working_dir: impl Into<PathBuf>) -> Result<LockedApp> {
    let working_dir = working_dir.into();
    LockedAppBuilder { working_dir }.build(app)
}

struct LockedAppBuilder {
    working_dir: PathBuf,
}

impl LockedAppBuilder {
    fn build(self, app: Application) -> Result<LockedApp> {
        Ok(LockedApp {
            spin_lock_version: spin_app::locked::FixedVersion,
            triggers: self.build_triggers(&app.info.trigger, app.component_triggers)?,
            metadata: self.build_metadata(app.info),
            variables: self.build_variables(app.variables)?,
            components: self.build_components(app.components)?,
        })
    }

    fn build_metadata(&self, info: ApplicationInformation) -> ValuesMap {
        let redis_address = match info.trigger {
            ApplicationTrigger::Redis(RedisTriggerConfiguration { address }) => Some(address),
            _ => None,
        };
        let origin = match info.origin {
            spin_manifest::ApplicationOrigin::File(path) => {
                format!("file://{}", path.to_string_lossy())
            }
            spin_manifest::ApplicationOrigin::Bindle { id, server } => {
                format!("bindle+{server}?id={id}")
            }
        };
        ValuesMapBuilder::new()
            .string("name", &info.name)
            .string("version", &info.version)
            .string_option("description", info.description.as_deref())
            .string_option("redis_address", redis_address)
            .string("origin", origin)
            .build()
    }

    fn build_variables<B: FromIterator<(String, locked::Variable)>>(
        &self,
        variables: impl IntoIterator<Item = (String, spin_manifest::Variable)>,
    ) -> Result<B> {
        variables
            .into_iter()
            .map(|(name, var)| {
                ensure!(
                    var.required ^ var.default.is_some(),
                    "variable {name:?} must either be required or have a default"
                );
                Ok((
                    name,
                    locked::Variable {
                        default: var.default,
                        secret: var.secret,
                    },
                ))
            })
            .collect()
    }

    fn build_triggers(
        &self,
        app_trigger: &ApplicationTrigger,
        component_triggers: impl IntoIterator<Item = (String, TriggerConfig)>,
    ) -> Result<Vec<LockedTrigger>> {
        component_triggers
            .into_iter()
            .map(|(component_id, config)| {
                let id = format!("trigger--{component_id}");

                let mut builder = ValuesMapBuilder::new();
                builder.string("component", component_id);
                let trigger_type = match (app_trigger, config) {
                    (ApplicationTrigger::Http(HttpTriggerConfiguration{base}), TriggerConfig::Http(HttpConfig{ route, executor })) => {
                        let route = base.trim_end_matches('/').to_string() + &route;
                        builder.string("route", route);                     
                        builder.serializable("executor", executor)?;
                        "http"
                    },
                    (ApplicationTrigger::Redis(_), TriggerConfig::Redis(RedisConfig{ channel, executor: _ })) => {
                        builder.string("channel", channel);
                        "redis"
                    },
                    (app_config, trigger_config) => bail!("Mismatched app and component trigger configs: {app_config:?} vs {trigger_config:?}")
                }.into();
                let trigger_config = builder.build().into();

                Ok(LockedTrigger {
                    id,
                    trigger_type,
                    trigger_config,
                })
            })
            .collect()
    }

    fn build_components(
        &self,
        components: impl IntoIterator<Item = CoreComponent>,
    ) -> Result<Vec<LockedComponent>> {
        components
            .into_iter()
            .map(|component| {
                let id = component.id;

                let metadata = ValuesMapBuilder::new()
                    .string_option("description", component.description)
                    .string_array("allowed_http_hosts", component.wasm.allowed_http_hosts)
                    .build();

                let source = {
                    let path = match component.source {
                        spin_manifest::ModuleSource::FileReference(path) => path,
                        spin_manifest::ModuleSource::Buffer(bytes, name) => {
                            let wasm_path = self.working_dir.join(&id).with_extension("wasm");
                            std::fs::write(&wasm_path, bytes).with_context(|| {
                                format!(
                                    "Failed to write buffered source for component {id:?} ({name})"
                                )
                            })?;
                            wasm_path
                        }
                    };
                    LockedComponentSource {
                        content_type: WASM_CONTENT_TYPE.into(),
                        content: content_file_path(path),
                    }
                };

                let env = component.wasm.environment.into_iter().collect();

                let files = component
                    .wasm
                    .mounts
                    .into_iter()
                    .map(|mount| ContentPath {
                        content: content_file_path(mount.host),
                        path: mount.guest.into(),
                    })
                    .collect();

                let config = component.config.into_iter().collect();

                Ok(LockedComponent {
                    id,
                    metadata,
                    source,
                    env,
                    files,
                    config,
                })
            })
            .collect()
    }
}

fn content_file_path(path: PathBuf) -> ContentRef {
    ContentRef {
        source: Some(format!("file://{}", path.to_string_lossy())),
        ..Default::default()
    }
}
