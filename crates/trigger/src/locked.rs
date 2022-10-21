#![allow(dead_code)] // Refactor WIP

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use spin_app::{
    locked::{
        self, ContentPath, ContentRef, LockedApp, LockedComponent, LockedComponentSource,
        LockedTrigger,
    },
    values::{ValuesMap, ValuesMapBuilder},
};
use spin_manifest::{
    Application, ApplicationInformation, ApplicationTrigger, CoreComponent, HttpConfig,
    HttpTriggerConfiguration, RedisConfig, TriggerConfig,
};

const WASM_CONTENT_TYPE: &str = "application/wasm";

/// Construct a LockedApp from the given Application. Any buffered component
/// sources will be written to the given `working_dir`.
pub fn build_locked_app(app: Application, working_dir: impl Into<PathBuf>) -> Result<LockedApp> {
    let working_dir = working_dir.into().canonicalize()?;
    LockedAppBuilder { working_dir }.build(app)
}

struct LockedAppBuilder {
    working_dir: PathBuf,
}

// TODO(lann): Consolidate metadata w/ spin-manifest models
impl LockedAppBuilder {
    fn build(self, app: Application) -> Result<LockedApp> {
        Ok(LockedApp {
            spin_lock_version: spin_app::locked::FixedVersion,
            triggers: self.build_triggers(&app.info.trigger, app.component_triggers)?,
            metadata: self.build_metadata(app.info)?,
            variables: self.build_variables(app.variables)?,
            components: self.build_components(app.components)?,
        })
    }

    fn build_metadata(&self, info: ApplicationInformation) -> Result<ValuesMap> {
        let mut builder = ValuesMapBuilder::new();
        builder
            .string("name", &info.name)
            .string("version", &info.version)
            .string_option("description", info.description.as_deref())
            .serializable("trigger", info.trigger)?;
        // Convert ApplicationOrigin to a URL
        builder.string(
            "origin",
            match info.origin {
                spin_manifest::ApplicationOrigin::File(path) => file_uri(&path)?,
                spin_manifest::ApplicationOrigin::Bindle { id, server } => {
                    format!("bindle+{server}?id={id}")
                }
            },
        );
        Ok(builder.build())
    }

    fn build_variables<B: FromIterator<(String, locked::Variable)>>(
        &self,
        variables: impl IntoIterator<Item = (String, spin_manifest::Variable)>,
    ) -> Result<B> {
        variables
            .into_iter()
            .map(|(name, var)| {
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

                let trigger_type;
                match (app_trigger, config) {
                    (ApplicationTrigger::Http(HttpTriggerConfiguration{base: _}), TriggerConfig::Http(HttpConfig{ route, executor })) => {
                        trigger_type = "http";
                        builder.string("route", route);
                        builder.serializable("executor", executor)?;
                    },
                    (ApplicationTrigger::Redis(_), TriggerConfig::Redis(RedisConfig{ channel, executor: _ })) => {
                        trigger_type = "redis";
                        builder.string("channel", channel);
                    },
                    (app_config, trigger_config) => bail!("Mismatched app and component trigger configs: {app_config:?} vs {trigger_config:?}")
                }

                Ok(LockedTrigger {
                    id,
                    trigger_type: trigger_type.into(),
                    trigger_config: builder.build().into()
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
            .map(|component| self.build_component(component))
            .collect()
    }

    fn build_component(&self, component: CoreComponent) -> Result<LockedComponent> {
        let id = component.id;

        let metadata = ValuesMapBuilder::new()
            .string_option("description", component.description)
            .string_array("allowed_http_hosts", component.wasm.allowed_http_hosts)
            .take();

        let source = {
            let path = match component.source {
                spin_manifest::ModuleSource::FileReference(path) => path,
                spin_manifest::ModuleSource::Buffer(bytes, name) => {
                    let wasm_path = self.working_dir.join(&id).with_extension("wasm");
                    std::fs::write(&wasm_path, bytes).with_context(|| {
                        format!("Failed to write buffered source for component {id:?} ({name})")
                    })?;
                    wasm_path
                }
            };
            LockedComponentSource {
                content_type: WASM_CONTENT_TYPE.into(),
                content: content_ref_path(&path)?,
            }
        };

        let env = component.wasm.environment.into_iter().collect();

        let files = component
            .wasm
            .mounts
            .into_iter()
            .map(|mount| {
                Ok(ContentPath {
                    content: content_ref_path(&mount.host)?,
                    path: mount.guest.into(),
                })
            })
            .collect::<Result<_>>()?;

        let config = component.config.into_iter().collect();

        Ok(LockedComponent {
            id,
            metadata,
            source,
            env,
            files,
            config,
        })
    }
}

fn content_ref_path(path: &Path) -> Result<ContentRef> {
    Ok(ContentRef {
        source: Some(
            file_uri(path).with_context(|| format!("failed to resolve content at {path:?}"))?,
        ),
        ..Default::default()
    })
}

fn file_uri(path: &Path) -> Result<String> {
    let path = path.canonicalize()?;
    let url = if path.is_dir() {
        url::Url::from_directory_path(&path)
    } else {
        url::Url::from_file_path(&path)
    }
    .map_err(|_| anyhow!("Could not construct file URL for {path:?}"))?;
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TEST_MANIFEST: &str = r#"
        spin_version = "1"
        name = "test-app"
        version = "0.0.1"
        trigger = { type = "http", base = "/" }

        [variables]
        test_var = { default = "test-val" }

        [[component]]
        id = "test-component"
        source = "test-source.wasm"
        files = ["static.txt"]
        allowed_http_hosts = ["example.com"]
        [component.config]
        test_config = "{{test_var}}"
        [component.trigger]
        route = "/"

        [[component]]
        id = "test-component-2"
        source = "test-source.wasm"
        allowed_http_hosts = ["insecure:allow-all"]
        [component.trigger]
        route = "/other"
    "#;

    async fn test_app() -> (Application, TempDir) {
        let tempdir = TempDir::new().expect("tempdir");
        std::env::set_current_dir(tempdir.path()).unwrap();
        std::fs::write("spin.toml", TEST_MANIFEST).expect("write manifest");
        std::fs::write("test-source.wasm", "not actual wasm").expect("write source");
        std::fs::write("static.txt", "content").expect("write static");
        let app = spin_loader::local::from_file("spin.toml", &tempdir, &None)
            .await
            .expect("load app");
        (app, tempdir)
    }

    #[tokio::test]
    async fn build_locked_app_smoke_test() {
        let (app, tempdir) = test_app().await;
        let locked = build_locked_app(app, tempdir.path()).unwrap();
        assert_eq!(locked.metadata["name"], "test-app");
        assert!(locked.variables.contains_key("test_var"));
        assert_eq!(locked.triggers[0].trigger_config["route"], "/");

        let component = &locked.components[0];
        let source = component.source.content.source.as_deref().unwrap();
        assert!(source.ends_with("test-source.wasm"));
        let mount = component.files[0].content.source.as_deref().unwrap();
        assert!(mount.ends_with('/'));
    }
}
