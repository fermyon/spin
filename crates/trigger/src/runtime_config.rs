pub mod client_tls;
pub mod key_value;
pub mod llm;
pub mod sqlite;
pub mod variables_provider;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use http::uri::Authority;
use serde::Deserialize;
use spin_common::ui::quoted_path;
use spin_sqlite::Connection;

use crate::TriggerHooks;

use self::{
    client_tls::{load_certs, load_key, ClientTlsOpts},
    key_value::{KeyValueStore, KeyValueStoreOpts},
    llm::LlmComputeOpts,
    sqlite::SqliteDatabaseOpts,
    variables_provider::{VariablesProvider, VariablesProviderOpts},
};

pub const DEFAULT_STATE_DIR: &str = ".spin";
const DEFAULT_LOGS_DIR: &str = "logs";
/// RuntimeConfig allows multiple sources of runtime configuration to be
/// queried uniformly.
#[derive(Debug, Default)]
pub struct RuntimeConfig {
    local_app_dir: Option<PathBuf>,
    files: Vec<RuntimeConfigOpts>,
    overrides: RuntimeConfigOpts,
}

impl RuntimeConfig {
    // Gives more consistent conditional branches
    #![allow(clippy::manual_map)]

    pub fn new(local_app_dir: Option<PathBuf>) -> Self {
        Self {
            local_app_dir,
            ..Default::default()
        }
    }

    /// Load a runtime config file from the given path. Options specified in a
    /// later-loaded file take precedence over any earlier-loaded files.
    pub fn merge_config_file(&mut self, path: impl Into<PathBuf>) -> Result<()> {
        let path = path.into();
        let mut opts = RuntimeConfigOpts::parse_file(&path)?;
        opts.file_path = Some(path);
        self.files.push(opts);
        Ok(())
    }

    /// Return a Vec of configured [`VariablesProvider`]s.
    pub fn variables_providers(&self) -> Vec<VariablesProvider> {
        let default_provider = VariablesProviderOpts::default_provider_opts(self).build_provider();
        let mut providers: Vec<VariablesProvider> = vec![default_provider];
        providers.extend(self.opts_layers().flat_map(|opts| {
            opts.variables_providers
                .iter()
                .map(|opts| opts.build_provider())
        }));
        providers
    }

    /// Return an iterator of named configured [`KeyValueStore`]s.
    pub fn key_value_stores(&self) -> Result<impl IntoIterator<Item = (String, KeyValueStore)>> {
        let mut stores = HashMap::new();
        // Insert explicitly-configured stores
        for opts in self.opts_layers() {
            for (name, store) in &opts.key_value_stores {
                if !stores.contains_key(name) {
                    let store = store.build_store(opts)?;
                    stores.insert(name.to_owned(), store);
                }
            }
        }
        // Upsert default store
        if !stores.contains_key("default") {
            let store = KeyValueStoreOpts::default_store_opts(self)
                .build_store(&RuntimeConfigOpts::default())?;
            stores.insert("default".into(), store);
        }
        Ok(stores.into_iter())
    }

    // Return the "default" key value store config.
    fn default_key_value_opts(&self) -> KeyValueStoreOpts {
        self.opts_layers()
            .find_map(|opts| opts.key_value_stores.get("default"))
            .cloned()
            .unwrap_or_else(|| KeyValueStoreOpts::default_store_opts(self))
    }

    // Return the "default" key value store config.
    fn default_sqlite_opts(&self) -> SqliteDatabaseOpts {
        self.opts_layers()
            .find_map(|opts| opts.sqlite_databases.get("default"))
            .cloned()
            .unwrap_or_else(|| SqliteDatabaseOpts::default(self))
    }

    /// Return an iterator of named configured [`SqliteDatabase`]s.
    pub async fn sqlite_databases(
        &self,
    ) -> Result<impl IntoIterator<Item = (String, Arc<dyn Connection>)>> {
        let mut databases = HashMap::new();
        // Insert explicitly-configured databases
        for opts in self.opts_layers() {
            for (name, database) in &opts.sqlite_databases {
                if !databases.contains_key(name) {
                    let store = database.build(opts).await?;
                    databases.insert(name.to_owned(), store);
                }
            }
        }
        // Upsert default store
        if !databases.contains_key("default") {
            let store = SqliteDatabaseOpts::default(self)
                .build(&RuntimeConfigOpts::default())
                .await?;
            databases.insert("default".into(), store);
        }
        Ok(databases.into_iter())
    }

    /// Set the state dir, overriding any other runtime config source.
    pub fn set_state_dir(&mut self, state_dir: impl Into<String>) {
        self.overrides.state_dir = Some(state_dir.into());
    }

    /// Return the state dir if set.
    pub fn state_dir(&self) -> Option<PathBuf> {
        if let Some(path_str) = self.find_opt(|opts| &opts.state_dir) {
            if path_str.is_empty() {
                None // An empty string forces the state dir to be unset
            } else {
                Some(path_str.into())
            }
        } else if let Some(app_dir) = &self.local_app_dir {
            // If we're running a local app, return the default state dir
            Some(app_dir.join(DEFAULT_STATE_DIR))
        } else {
            None
        }
    }

    /// Set the log dir, overriding any other runtime config source.
    pub fn set_log_dir(&mut self, log_dir: impl Into<PathBuf>) {
        self.overrides.log_dir = Some(log_dir.into());
    }

    /// Return the log dir if set.
    pub fn log_dir(&self) -> Option<PathBuf> {
        if let Some(path) = self.find_opt(|opts| &opts.log_dir) {
            if path.as_os_str().is_empty() {
                // If the log dir is explicitly set to "", disable logging
                None
            } else {
                // If there is an explicit log dir set, return it
                Some(path.into())
            }
        } else if let Some(state_dir) = self.state_dir() {
            // If the state dir is set, build the default path
            Some(state_dir.join(DEFAULT_LOGS_DIR))
        } else {
            None
        }
    }

    pub fn llm_compute(&self) -> &LlmComputeOpts {
        if let Some(compute) = self.find_opt(|opts| &opts.llm_compute) {
            compute
        } else {
            &LlmComputeOpts::Spin
        }
    }

    // returns the client tls options in form of nested
    // HashMap of { Component ID -> HashMap of { Host -> ParsedClientTlsOpts} }
    pub fn client_tls_opts(
        &self,
    ) -> Result<HashMap<String, HashMap<Authority, ParsedClientTlsOpts>>> {
        let mut components_map: HashMap<String, HashMap<Authority, ParsedClientTlsOpts>> =
            HashMap::new();

        for opt_layer in self.opts_layers() {
            for opts in &opt_layer.client_tls_opts {
                let parsed = parse_client_tls_opts(opts).context("parsing client tls options")?;
                for component_id in &opts.component_ids {
                    let hostmap = parsed
                        .hosts
                        .clone()
                        .into_iter()
                        .map(|host| (host, parsed.clone()))
                        .collect::<HashMap<Authority, ParsedClientTlsOpts>>();
                    components_map.insert(component_id.to_owned(), hostmap);
                }
            }
        }

        Ok(components_map)
    }

    /// Returns an iterator of RuntimeConfigOpts in order of decreasing precedence
    fn opts_layers(&self) -> impl Iterator<Item = &RuntimeConfigOpts> {
        std::iter::once(&self.overrides).chain(self.files.iter().rev())
    }

    /// Returns the highest precedence RuntimeConfigOpts Option that is set
    fn find_opt<T>(&self, mut f: impl FnMut(&RuntimeConfigOpts) -> &Option<T>) -> Option<&T> {
        self.opts_layers().find_map(|opts| f(opts).as_ref())
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfigOpts {
    #[serde(default)]
    pub state_dir: Option<String>,

    #[serde(default)]
    pub log_dir: Option<PathBuf>,

    #[serde(default)]
    pub llm_compute: Option<LlmComputeOpts>,

    #[serde(rename = "variables_provider", alias = "config_provider", default)]
    pub variables_providers: Vec<VariablesProviderOpts>,

    #[serde(rename = "key_value_store", default)]
    pub key_value_stores: HashMap<String, KeyValueStoreOpts>,

    #[serde(rename = "sqlite_database", default)]
    pub sqlite_databases: HashMap<String, SqliteDatabaseOpts>,

    #[serde(skip)]
    pub file_path: Option<PathBuf>,

    #[serde(rename = "client_tls", default)]
    pub client_tls_opts: Vec<ClientTlsOpts>,
}

impl RuntimeConfigOpts {
    fn parse_file(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read runtime config file {}", quoted_path(path)))?;
        let ext = path.extension().unwrap_or_default();
        let is_json = ext != "toml" && (ext == "json" || contents.trim_start().starts_with('{'));
        if is_json {
            serde_json::from_str(&contents).with_context(|| {
                format!(
                    "Failed to parse runtime config JSON file {}",
                    quoted_path(path)
                )
            })
        } else {
            toml::from_str(&contents).with_context(|| {
                format!(
                    "Failed to parse runtime config TOML file {}",
                    quoted_path(path)
                )
            })
        }
    }
}

fn resolve_config_path(path: &Path, config_opts: &RuntimeConfigOpts) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_owned());
    }
    let base_path = match &config_opts.file_path {
        Some(file_path) => file_path
            .parent()
            .with_context(|| {
                format!(
                    "failed to get parent of runtime config file path {}",
                    quoted_path(file_path)
                )
            })?
            .to_owned(),
        None => std::env::current_dir().context("failed to get current directory")?,
    };
    Ok(base_path.join(path))
}

pub(crate) struct SummariseRuntimeConfigHook {
    runtime_config_file: Option<PathBuf>,
}

impl SummariseRuntimeConfigHook {
    pub(crate) fn new(runtime_config_file: &Option<PathBuf>) -> Self {
        Self {
            runtime_config_file: runtime_config_file.clone(),
        }
    }
}

impl TriggerHooks for SummariseRuntimeConfigHook {
    fn app_loaded(
        &mut self,
        _app: &spin_app::App,
        runtime_config: &RuntimeConfig,
        _resolver: &Arc<spin_expressions::PreparedResolver>,
    ) -> anyhow::Result<()> {
        if let Some(path) = &self.runtime_config_file {
            let mut opts = vec![];
            for opt in runtime_config.opts_layers() {
                for (id, opt) in &opt.key_value_stores {
                    opts.push(Self::summarise_kv(id, opt));
                }
                for (id, opt) in &opt.sqlite_databases {
                    opts.push(Self::summarise_sqlite(id, opt));
                }
                for opt in &opt.llm_compute {
                    opts.push(Self::summarise_llm(opt));
                }
            }
            if !opts.is_empty() {
                let opts_text = opts.join(", ");
                println!(
                    "Using {opts_text} runtime config from {}",
                    quoted_path(path)
                );
            }
        }
        Ok(())
    }
}

impl SummariseRuntimeConfigHook {
    fn summarise_kv(id: &str, opt: &KeyValueStoreOpts) -> String {
        let source = match opt {
            KeyValueStoreOpts::Spin(_) => "spin",
            KeyValueStoreOpts::Redis(_) => "redis",
            KeyValueStoreOpts::AzureCosmos(_) => "cosmos",
        };
        format!("[key_value_store.{id}: {}]", source)
    }

    fn summarise_sqlite(id: &str, opt: &SqliteDatabaseOpts) -> String {
        let source = match opt {
            SqliteDatabaseOpts::Spin(_) => "spin",
            SqliteDatabaseOpts::Libsql(_) => "libsql",
        };
        format!("[sqlite_database.{id}: {}]", source)
    }

    fn summarise_llm(opt: &LlmComputeOpts) -> String {
        let source = match opt {
            LlmComputeOpts::Spin => "spin",
            LlmComputeOpts::RemoteHttp(_) => "remote-http",
        };
        format!("[llm_compute: {}]", source)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;
    use toml::toml;

    use super::*;

    #[test]
    fn defaults_without_local_app_dir() -> Result<()> {
        let config = RuntimeConfig::new(None);

        assert_eq!(config.state_dir(), None);
        assert_eq!(config.log_dir(), None);
        assert_eq!(default_spin_store_path(&config), None);

        Ok(())
    }

    #[test]
    fn defaults_with_local_app_dir() -> Result<()> {
        let app_dir = tempfile::tempdir()?;
        let config = RuntimeConfig::new(Some(app_dir.path().into()));

        let state_dir = config.state_dir().unwrap();
        assert!(state_dir.starts_with(&app_dir));

        let log_dir = config.log_dir().unwrap();
        assert!(log_dir.starts_with(&state_dir));

        let default_db_path = default_spin_store_path(&config).unwrap();
        assert!(default_db_path.starts_with(&state_dir));

        Ok(())
    }

    #[test]
    fn state_dir_force_unset() -> Result<()> {
        let app_dir = tempfile::tempdir()?;
        let mut config = RuntimeConfig::new(Some(app_dir.path().into()));
        assert!(config.state_dir().is_some());

        config.set_state_dir("");
        assert!(config.state_dir().is_none());

        Ok(())
    }

    #[test]
    fn opts_layers_precedence() -> Result<()> {
        let mut config = RuntimeConfig::new(None);

        merge_config_toml(
            &mut config,
            toml! {
                state_dir = "file-state-dir"
                log_dir = "file-log-dir"
            },
        );

        let state_dir = config.state_dir().unwrap();
        assert_eq!(state_dir.as_os_str(), "file-state-dir");

        let log_dir = config.log_dir().unwrap();
        assert_eq!(log_dir.as_os_str(), "file-log-dir");

        config.set_state_dir("override-state-dir");
        config.set_log_dir("override-log-dir");

        let state_dir = config.state_dir().unwrap();
        assert_eq!(state_dir.as_os_str(), "override-state-dir");

        let log_dir = config.log_dir().unwrap();
        assert_eq!(log_dir.as_os_str(), "override-log-dir");

        Ok(())
    }

    #[test]
    fn deprecated_config_provider_in_runtime_config_file() -> Result<()> {
        let mut config = RuntimeConfig::new(None);

        // One default provider
        assert_eq!(config.variables_providers().len(), 1);

        merge_config_toml(
            &mut config,
            toml! {
                [[config_provider]]
                type = "vault"
                url = "http://vault"
                token = "secret"
                mount = "root"
            },
        );
        assert_eq!(config.variables_providers().len(), 2);

        Ok(())
    }

    #[test]
    fn variables_providers_from_file() -> Result<()> {
        let mut config = RuntimeConfig::new(None);

        // One default provider
        assert_eq!(config.variables_providers().len(), 1);

        merge_config_toml(
            &mut config,
            toml! {
                [[variables_provider]]
                type = "vault"
                url = "http://vault"
                token = "secret"
                mount = "root"
            },
        );
        assert_eq!(config.variables_providers().len(), 2);

        Ok(())
    }

    #[test]
    fn key_value_stores_from_file() -> Result<()> {
        let mut config = RuntimeConfig::new(None);

        // One default store
        assert_eq!(config.key_value_stores().unwrap().into_iter().count(), 1);

        merge_config_toml(
            &mut config,
            toml! {
                [key_value_store.default]
                type = "spin"
                path = "override.db"

                [key_value_store.other]
                type = "spin"
                path = "other.db"
            },
        );
        assert_eq!(config.key_value_stores().unwrap().into_iter().count(), 2);

        Ok(())
    }

    #[test]
    fn default_redis_key_value_store_from_file() -> Result<()> {
        let mut config = RuntimeConfig::new(None);

        merge_config_toml(
            &mut config,
            toml! {
                [key_value_store.default]
                type = "redis"
                url = "redis://127.0.0.1/"
            },
        );
        assert_eq!(config.key_value_stores().unwrap().into_iter().count(), 1);

        assert!(
            matches!(config.default_key_value_opts(), KeyValueStoreOpts::Redis(_)),
            "expected default Redis store",
        );

        Ok(())
    }

    #[test]
    fn test_parsing_valid_hosts_in_client_opts() {
        let input = ClientTlsOpts {
            component_ids: vec!["component-id-foo".to_string()],
            hosts: vec!["fermyon.com".to_string(), "fermyon.com:5443".to_string()],
            ca_roots_file: None,
            cert_chain_file: None,
            private_key_file: None,
            ca_webpki_roots: None,
        };

        let parsed = parse_client_tls_opts(&input);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().hosts.len(), 2)
    }

    #[test]
    fn test_parsing_empty_hosts_in_client_opts() {
        let input = ClientTlsOpts {
            component_ids: vec!["component-id-foo".to_string()],
            hosts: vec!["".to_string(), "fermyon.com:5443".to_string()],
            ca_roots_file: None,
            cert_chain_file: None,
            private_key_file: None,
            ca_webpki_roots: None,
        };

        let parsed = parse_client_tls_opts(&input);
        assert!(parsed.is_err());
        assert_eq!(
            "failed to parse uri ''. error: InvalidUri(Empty)",
            parsed.unwrap_err().to_string()
        )
    }

    #[test]
    fn test_parsing_invalid_hosts_in_client_opts() {
        let input = ClientTlsOpts {
            component_ids: vec!["component-id-foo".to_string()],
            hosts: vec!["perc%ent:443".to_string(), "fermyon.com:5443".to_string()],
            ca_roots_file: None,
            cert_chain_file: None,
            private_key_file: None,
            ca_webpki_roots: None,
        };

        let parsed = parse_client_tls_opts(&input);
        assert!(parsed.is_err());
        assert_eq!(
            "failed to parse uri 'perc%ent:443'. error: InvalidUri(InvalidAuthority)",
            parsed.unwrap_err().to_string()
        )
    }

    fn merge_config_toml(config: &mut RuntimeConfig, value: toml::Value) {
        let data = toml::to_vec(&value).expect("encode toml");
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(&data).expect("write toml");
        config.merge_config_file(file.path()).expect("merge config");
    }

    fn default_spin_store_path(config: &RuntimeConfig) -> Option<PathBuf> {
        match config.default_key_value_opts() {
            KeyValueStoreOpts::Spin(opts) => opts.path,
            other => panic!("unexpected default store opts {other:?}"),
        }
    }
}

// parsed client tls options
#[derive(Debug, Clone)]
pub struct ParsedClientTlsOpts {
    pub components: Vec<String>,
    pub hosts: Vec<Authority>,
    pub custom_root_ca: Option<Vec<rustls_pki_types::CertificateDer<'static>>>,
    pub cert_chain: Option<Vec<rustls_pki_types::CertificateDer<'static>>>,
    pub private_key: Option<Arc<rustls_pki_types::PrivateKeyDer<'static>>>,
    pub ca_webpki_roots: Option<bool>,
}

fn parse_client_tls_opts(inp: &ClientTlsOpts) -> Result<ParsedClientTlsOpts, anyhow::Error> {
    let custom_root_ca = match &inp.ca_roots_file {
        Some(path) => Some(load_certs(path).context("loading custom root ca")?),
        None => None,
    };

    let cert_chain = match &inp.cert_chain_file {
        Some(file) => Some(load_certs(file).context("loading client tls certs")?),
        None => None,
    };

    let private_key = match &inp.private_key_file {
        Some(file) => {
            let privatekey = load_key(file).context("loading private key")?;
            Some(Arc::from(privatekey))
        }
        None => None,
    };

    let parsed_hosts: Vec<Authority> = inp
        .hosts
        .clone()
        .into_iter()
        .map(|s| {
            s.parse::<Authority>()
                .map_err(|e| anyhow::anyhow!("failed to parse uri '{}'. error: {:?}", s, e))
        })
        .collect::<Result<Vec<Authority>, anyhow::Error>>()?;

    Ok(ParsedClientTlsOpts {
        hosts: parsed_hosts,
        components: inp.component_ids.clone(),
        custom_root_ca,
        cert_chain,
        private_key,
        ca_webpki_roots: inp.ca_webpki_roots,
    })
}
