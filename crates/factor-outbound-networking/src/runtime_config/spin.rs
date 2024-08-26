use anyhow::{bail, ensure, Context};
use serde::{Deserialize, Deserializer};
use spin_factors::runtime_config::toml::GetTomlValue;
use std::io;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{validate_host, TlsConfig};

/// Spin's default handling of the runtime configuration for outbound TLS.
pub struct SpinTlsRuntimeConfig {
    runtime_config_dir: PathBuf,
}

impl SpinTlsRuntimeConfig {
    /// Creates a new `SpinTlsRuntimeConfig`.
    ///
    /// The given `runtime_config_dir` will be used as the root to resolve any
    /// relative paths.
    pub fn new(runtime_config_dir: impl Into<PathBuf>) -> Self {
        Self {
            runtime_config_dir: runtime_config_dir.into(),
        }
    }

    /// Get the runtime configuration for client TLS from a TOML table.
    ///
    /// Expects table to be in the format:
    /// ````toml
    /// [[client_tls]]
    /// component_ids = ["example-component"]
    /// hosts = ["example.com"]
    /// ca_use_webpki_roots = true
    /// ca_roots_file = "path/to/roots.crt"
    /// client_cert_file = "path/to/client.crt"
    /// client_private_key_file = "path/to/client.key"
    /// ```
    pub fn config_from_table(
        &self,
        table: &impl GetTomlValue,
    ) -> anyhow::Result<Option<super::RuntimeConfig>> {
        let Some(tls_configs) = self.tls_configs_from_table(table)? else {
            return Ok(None);
        };
        let runtime_config = super::RuntimeConfig::new(tls_configs)?;
        Ok(Some(runtime_config))
    }

    fn tls_configs_from_table<T: GetTomlValue>(
        &self,
        table: &T,
    ) -> anyhow::Result<Option<Vec<TlsConfig>>> {
        let Some(array) = table.get("client_tls") else {
            return Ok(None);
        };
        let toml_configs: Vec<RuntimeConfigToml> = array.clone().try_into()?;

        let tls_configs = toml_configs
            .into_iter()
            .map(|toml_config| self.load_tls_config(toml_config))
            .collect::<anyhow::Result<Vec<_>>>()
            .context("failed to parse TLS configs from TOML")?;
        Ok(Some(tls_configs))
    }

    fn load_tls_config(&self, toml_config: RuntimeConfigToml) -> anyhow::Result<TlsConfig> {
        let RuntimeConfigToml {
            component_ids,
            hosts,
            ca_use_webpki_roots,
            ca_roots_file,
            client_cert_file,
            client_private_key_file,
        } = toml_config;
        ensure!(
            !component_ids.is_empty(),
            "[[client_tls]] 'component_ids' list may not be empty"
        );
        ensure!(
            !hosts.is_empty(),
            "[[client_tls]] 'hosts' list may not be empty"
        );

        let components = component_ids.into_iter().map(Into::into).collect();

        let hosts = hosts
            .iter()
            .map(|host| {
                host.parse()
                    .map_err(|err| anyhow::anyhow!("invalid host {host:?}: {err:?}"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let use_webpki_roots = if let Some(ca_use_webpki_roots) = ca_use_webpki_roots {
            ca_use_webpki_roots
        } else {
            // Use webpki roots by default *unless* explicit roots were given
            ca_roots_file.is_none()
        };

        let root_certificates = ca_roots_file
            .map(|path| self.load_certs(path))
            .transpose()?
            .unwrap_or_default();

        let client_cert = match (client_cert_file, client_private_key_file) {
            (Some(cert_path), Some(key_path)) => Some(super::ClientCertConfig {
                cert_chain: self.load_certs(cert_path)?,
                key_der: self.load_key(key_path)?,
            }),
            (None, None) => None,
            (Some(_), None) => bail!("client_cert_file specified without client_private_key_file"),
            (None, Some(_)) => bail!("client_private_key_file specified without client_cert_file"),
        };

        Ok(TlsConfig {
            components,
            hosts,
            root_certificates,
            use_webpki_roots,
            client_cert,
        })
    }

    // Parse certs from the provided file
    fn load_certs(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<Vec<rustls_pki_types::CertificateDer<'static>>> {
        let path = self.runtime_config_dir.join(path);
        rustls_pemfile::certs(&mut io::BufReader::new(fs::File::open(path).map_err(
            |err| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("failed to read cert file {:?}", err),
                )
            },
        )?))
        .collect()
    }

    // Parse a private key from the provided file
    fn load_key(
        &self,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<rustls_pki_types::PrivateKeyDer<'static>> {
        let path = self.runtime_config_dir.join(path);
        let file = fs::File::open(&path)
            .with_context(|| format!("failed to read private key from '{}'", path.display()))?;
        Ok(rustls_pemfile::private_key(&mut io::BufReader::new(file))
            .with_context(|| format!("failed to parse private key from '{}'", path.display()))?
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "private key file '{}' contains no private keys",
                        path.display()
                    ),
                )
            })?)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfigToml {
    component_ids: Vec<spin_serde::KebabId>,
    #[serde(deserialize_with = "deserialize_hosts")]
    hosts: Vec<String>,
    ca_use_webpki_roots: Option<bool>,
    ca_roots_file: Option<PathBuf>,
    client_cert_file: Option<PathBuf>,
    client_private_key_file: Option<PathBuf>,
}

fn deserialize_hosts<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let hosts = Vec::<String>::deserialize(deserializer)?;
    for host in &hosts {
        validate_host(host).map_err(serde::de::Error::custom)?;
    }
    Ok(hosts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TESTDATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata");

    #[test]
    fn test_min_config() -> anyhow::Result<()> {
        let config = SpinTlsRuntimeConfig::new("/doesnt-matter");

        let tls_configs = config
            .tls_configs_from_table(&toml::toml! {
                [[client_tls]]
                component_ids = ["test-component"]
                hosts = ["test-host"]

            })?
            .context("missing config section")?;
        assert_eq!(tls_configs.len(), 1);

        assert_eq!(tls_configs[0].components, ["test-component"]);
        assert_eq!(tls_configs[0].hosts[0].as_str(), "test-host");
        assert!(tls_configs[0].use_webpki_roots);
        Ok(())
    }

    #[test]
    fn test_max_config() -> anyhow::Result<()> {
        let config = SpinTlsRuntimeConfig::new(TESTDATA_DIR);

        let tls_configs = config
            .tls_configs_from_table(&toml::toml! {
                [[client_tls]]
                component_ids = ["test-component"]
                hosts = ["test-host"]
                ca_use_webpki_roots = true
                ca_roots_file = "valid-cert.pem"
                client_cert_file = "valid-cert.pem"
                client_private_key_file = "valid-private-key.pem"
            })?
            .context("missing config section")?;
        assert_eq!(tls_configs.len(), 1);

        assert!(tls_configs[0].use_webpki_roots);
        assert_eq!(tls_configs[0].root_certificates.len(), 2);
        assert!(tls_configs[0].client_cert.is_some());
        Ok(())
    }

    #[test]
    fn test_use_webpki_roots_default_with_explicit_roots() -> anyhow::Result<()> {
        let config = SpinTlsRuntimeConfig::new(TESTDATA_DIR);

        let tls_configs = config
            .tls_configs_from_table(&toml::toml! {
                [[client_tls]]
                component_ids = ["test-component"]
                hosts = ["test-host"]
                ca_roots_file = "valid-cert.pem"
            })?
            .context("missing config section")?;

        assert!(!tls_configs[0].use_webpki_roots);
        Ok(())
    }

    #[test]
    fn test_invalid_cert() {
        let config = SpinTlsRuntimeConfig::new(TESTDATA_DIR);

        config
            .tls_configs_from_table(&toml::toml! {
                [[client_tls]]
                component_ids = ["test-component"]
                hosts = ["test-host"]
                ca_roots_file = "invalid-cert.pem"
            })
            .unwrap_err();
    }

    #[test]
    fn test_invalid_private_key() {
        let config = SpinTlsRuntimeConfig::new(TESTDATA_DIR);

        config
            .tls_configs_from_table(&toml::toml! {
                [[client_tls]]
                component_ids = ["test-component"]
                hosts = ["test-host"]
                client_cert_file = "valid-cert.pem"
                client_private_key_file = "invalid-key.pem"
            })
            .unwrap_err();
    }
}
