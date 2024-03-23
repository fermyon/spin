use crate::tls::rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_rustls::{rustls, TlsAcceptor};

/// TLS configuration for the server.
#[derive(Clone)]
pub struct TlsConfig {
    /// Path to TLS certificate.
    pub cert_path: PathBuf,
    /// Path to TLS key.
    pub key_path: PathBuf,
}

impl TlsConfig {
    // Creates a TLS acceptor from server config.
    pub(super) fn server_config(&self) -> anyhow::Result<TlsAcceptor> {
        let certs = load_certs(&self.cert_path)?;
        let mut keys = load_keys(&self.key_path)?;

        let cfg = rustls::ServerConfig::builder()
            .with_safe_default_protocol_versions()
            .with_no_client_auth()
            .with_single_cert(
                certs,
                tokio_rustls::rustls::pki_types::PrivateKeyDer::Pkcs8(keys.remove(0)),
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(Arc::new(cfg).into())
    }
}

// Loads public certificate from file.
fn load_certs(path: impl AsRef<Path>) -> io::Result<Vec<CertificateDer<'static>>> {
    certs(&mut io::BufReader::new(fs::File::open(path)?)).collect()
}

// Loads private key from file.
fn load_keys(path: impl AsRef<Path>) -> io::Result<Vec<PrivatePkcs8KeyDer<'static>>> {
    pkcs8_private_keys(&mut io::BufReader::new(fs::File::open(path)?)).collect()
}
