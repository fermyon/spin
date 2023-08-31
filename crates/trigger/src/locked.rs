#![allow(dead_code)] // Refactor WIP

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use outbound_http::ALLOWED_HTTP_HOSTS_KEY;
use spin_app::{
    locked::{
        self, ContentPath, ContentRef, LockedApp, LockedComponent, LockedComponentSource,
        LockedTrigger,
    },
    values::{ValuesMap, ValuesMapBuilder},
    MetadataKey,
};
use spin_key_value::KEY_VALUE_STORES_KEY;
use spin_manifest::{
    Application, ApplicationInformation, ApplicationOrigin, ApplicationTrigger, CoreComponent,
    HttpConfig, HttpTriggerConfiguration, RedisConfig, TriggerConfig,
};
use spin_sqlite::DATABASES_KEY;

pub const NAME_KEY: MetadataKey = MetadataKey::new("name");
pub const VERSION_KEY: MetadataKey = MetadataKey::new("version");
pub const DESCRIPTION_KEY: MetadataKey = MetadataKey::new("description");
pub const BINDLE_VERSION_KEY: MetadataKey = MetadataKey::new("bindle_version");
pub const ORIGIN_KEY: MetadataKey = MetadataKey::new("origin");

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
            .string(NAME_KEY, &info.name)
            .string(VERSION_KEY, &info.version)
            .string_option(DESCRIPTION_KEY, info.description.as_deref())
            .serializable("trigger", info.trigger)?;
        // Convert ApplicationOrigin to a URL
        let origin = match info.origin {
            ApplicationOrigin::File(path) => file_uri(&path)?,
            ApplicationOrigin::Bindle { id, server } => {
                if let Some((_, version)) = id.split_once('/') {
                    builder.string(BINDLE_VERSION_KEY, version);
                }
                format!("bindle+{server}?id={id}")
            }
        };
        builder.string(ORIGIN_KEY, origin);
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
                    (ApplicationTrigger::External(c), TriggerConfig::External(t)) => {
                        trigger_type = c.trigger_type();
                        for (key, value) in &t {
                            builder.serializable(key, value)?;
                        }
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
            .string_option(DESCRIPTION_KEY, component.description)
            .string_array(ALLOWED_HTTP_HOSTS_KEY, component.wasm.allowed_http_hosts)
            .string_array(KEY_VALUE_STORES_KEY, component.wasm.key_value_stores)
            .string_array(DATABASES_KEY, component.wasm.sqlite_databases)
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
    let url = url::Url::from_file_path(path)
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
        let dir = tempdir.path();

        std::fs::write(dir.join("spin.toml"), TEST_MANIFEST).expect("write manifest");
        std::fs::write(dir.join("test-source.wasm"), "not actual wasm").expect("write source");
        std::fs::write(dir.join("static.txt"), "content").expect("write static");
        let app = spin_loader::local::from_file(dir.join("spin.toml"), Some(&tempdir))
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
        let mount_path = url::Url::try_from(mount).unwrap().to_file_path().unwrap();
        assert!(mount_path.is_dir(), "{mount:?} is not a dir");
    }

    #[tokio::test]
    async fn lock_preserves_built_in_trigger_settings() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path();

        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/triggers");
        let app = spin_loader::from_file(base_dir.join("http.toml"), Some(dir))
            .await
            .unwrap();
        let locked = build_locked_app(app, dir).unwrap();

        assert_eq!("http", locked.metadata["trigger"]["type"]);
        assert_eq!("/test", locked.metadata["trigger"]["base"]);

        let tspin = locked
            .triggers
            .iter()
            .find(|t| t.id == "trigger--http-spin")
            .unwrap();
        assert_eq!("http", tspin.trigger_type);
        assert_eq!("http-spin", tspin.trigger_config["component"]);
        assert_eq!("/hello/...", tspin.trigger_config["route"]);

        let twagi = locked
            .triggers
            .iter()
            .find(|t| t.id == "trigger--http-wagi")
            .unwrap();
        assert_eq!("http", twagi.trigger_type);
        assert_eq!("http-wagi", twagi.trigger_config["component"]);
        assert_eq!("/waggy/...", twagi.trigger_config["route"]);
    }

    #[tokio::test]
    async fn lock_preserves_unknown_trigger_settings() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path();

        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/triggers");
        let app = spin_loader::from_file(base_dir.join("pounce.toml"), Some(dir))
            .await
            .unwrap();
        let locked = build_locked_app(app, dir).unwrap();

        assert_eq!("pounce", locked.metadata["trigger"]["type"]);
        assert_eq!("hobbes", locked.metadata["trigger"]["attacker"]);
        assert_eq!(1, locked.metadata["trigger"]["attackers-age"]);

        // Distinct settings make it across okay
        let t1 = locked
            .triggers
            .iter()
            .find(|t| t.id == "trigger--conf1")
            .unwrap();
        assert_eq!("pounce", t1.trigger_type);
        assert_eq!("conf1", t1.trigger_config["component"]);
        assert_eq!("MY KNEES", t1.trigger_config["on"]);
        assert_eq!(7, t1.trigger_config["sharpness"]);

        // Settings that could be mistaken for built-in make is across okay
        let t2 = locked
            .triggers
            .iter()
            .find(|t| t.id == "trigger--conf2")
            .unwrap();
        assert_eq!("pounce", t2.trigger_type);
        assert_eq!("conf2", t2.trigger_config["component"]);
        assert_eq!(
            "over the cat tree and out of the sun",
            t2.trigger_config["route"]
        );
    }
}
