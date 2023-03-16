use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use spin_config::provider::{env::EnvProvider, vault::VaultProvider};
use toml;

pub const DEFAULT_STATE_DIR: &str = ".spin";
const DEFAULT_LOGS_DIR: &str = "logs";

const SPIN_CONFIG_ENV_PREFIX: &str = "SPIN_CONFIG";

const DEFAULT_SQLITE_DB_FILENAME: &str = "sqlite_key_value.db";

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
            local_app_dir: local_app_dir.map(Into::into),
            ..Default::default()
        }
    }

    /// Load a runtime config file from the given path. Options specified in a
    /// later-loaded file take precedence over any earlier-loaded files.
    pub fn merge_config_from(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to load runtime config file {path:?}"))?;
        let opts = toml::from_slice(&bytes)
            .with_context(|| format!("Failed to parse runtime config file {path:?}"))?;
        self.files.push(opts);
        Ok(())
    }

    /// Return an iterator of configured spin_config::Providers.
    pub fn config_providers(&self) -> Vec<BoxedConfigProvider> {
        // Default EnvProvider
        let dotenv_path = self.local_app_dir.as_deref().map(|path| path.join(".env"));
        let env_provider = EnvProvider::new(SPIN_CONFIG_ENV_PREFIX, dotenv_path);

        let mut providers: Vec<BoxedConfigProvider> = vec![Box::new(env_provider)];
        providers.extend(self.opts_layers().flat_map(|opts| {
            opts.config_providers
                .iter()
                .map(Self::build_config_provider)
        }));
        providers
    }

    fn build_config_provider(provider_config: &ConfigProvider) -> BoxedConfigProvider {
        match provider_config {
            ConfigProvider::Vault(VaultConfig {
                url,
                token,
                mount,
                prefix,
            }) => Box::new(VaultProvider::new(url, token, mount, prefix.as_deref())),
        }
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
            // If there is an explicit log dir set, return it
            Some(path.into())
        } else if let Some(state_dir) = self.state_dir() {
            // If the state dir is set, build the default path
            Some(state_dir.join(DEFAULT_LOGS_DIR))
        } else {
            None
        }
    }

    /// Return a path to the sqlite DB if set.
    pub fn sqlite_db_path(&self) -> Option<PathBuf> {
        if let Some(state_dir) = self.state_dir() {
            // If the state dir is set, build the default path
            Some(state_dir.join(DEFAULT_SQLITE_DB_FILENAME))
        } else {
            None
        }
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
struct RuntimeConfigOpts {
    #[serde(default)]
    pub state_dir: Option<String>,

    #[serde(default)]
    pub log_dir: Option<PathBuf>,

    #[serde(rename = "config_provider", default)]
    pub config_providers: Vec<ConfigProvider>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ConfigProvider {
    Vault(VaultConfig),
}

// Vault config to initialize vault provider
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultConfig {
    pub url: String,
    pub token: String,
    pub mount: String,
    pub prefix: Option<String>,
}

type BoxedConfigProvider = Box<dyn spin_config::Provider>;

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
        assert_eq!(config.sqlite_db_path(), None);

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

        let sqlite_db_path = config.sqlite_db_path().unwrap();
        assert!(sqlite_db_path.starts_with(&state_dir));

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

        config.merge_config_from(toml_tempfile(toml! {
            state_dir = "file-state-dir"
            log_dir = "file-log-dir"
        })?)?;

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
    fn config_providers_from_file() -> Result<()> {
        let mut config = RuntimeConfig::new(None);

        // One default provider
        assert_eq!(config.config_providers().len(), 1);

        config.merge_config_from(toml_tempfile(toml! {
            [[config_provider]]
            type = "vault"
            url = "http://vault"
            token = "secret"
            mount = "root"
        })?)?;
        assert_eq!(config.config_providers().len(), 2);

        Ok(())
    }

    fn toml_tempfile(value: toml::Value) -> Result<NamedTempFile> {
        let data = toml::to_vec(&value)?;
        let mut file = NamedTempFile::new()?;
        file.write_all(&data)?;
        Ok(file)
    }
}
