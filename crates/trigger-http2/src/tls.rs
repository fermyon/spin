use rustls_pemfile::private_key;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_rustls::{rustls, TlsAcceptor};

// TODO: dedupe with spin-factor-outbound-networking (spin-tls crate?)

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
        let private_key = load_key(&self.key_path)?;

        let cfg = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, private_key)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(Arc::new(cfg).into())
    }
}

// load_certs parse and return the certs from the provided file
fn load_certs(
    path: impl AsRef<Path>,
) -> io::Result<Vec<rustls_pki_types::CertificateDer<'static>>> {
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

// parse and return the first private key from the provided file
fn load_key(path: impl AsRef<Path>) -> io::Result<rustls_pki_types::PrivateKeyDer<'static>> {
    private_key(&mut io::BufReader::new(fs::File::open(path).map_err(
        |err| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("failed to read private key file {:?}", err),
            )
        },
    )?))
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid private key"))
    .transpose()
    .ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "private key file contains no private keys",
        )
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    const TESTDATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata");

    #[test]
    fn test_read_non_existing_cert() {
        let path = Path::new(TESTDATA_DIR).join("non-existing-file.pem");

        let certs = load_certs(path);
        assert!(certs.is_err());
        assert_eq!(certs.err().unwrap().to_string(), "failed to read cert file Os { code: 2, kind: NotFound, message: \"No such file or directory\" }");
    }

    #[test]
    fn test_read_invalid_cert() {
        let path = Path::new(TESTDATA_DIR).join("invalid-cert.pem");

        let certs = load_certs(path);
        assert!(certs.is_err());
        assert_eq!(
            certs.err().unwrap().to_string(),
            "section end \"-----END CERTIFICATE-----\" missing"
        );
    }

    #[test]
    fn test_read_valid_cert() {
        let path = Path::new(TESTDATA_DIR).join("valid-cert.pem");

        let certs = load_certs(path);
        assert!(certs.is_ok());
        assert_eq!(certs.unwrap().len(), 2);
    }

    #[test]
    fn test_read_non_existing_private_key() {
        let path = Path::new(TESTDATA_DIR).join("non-existing-file.pem");

        let keys = load_key(path);
        assert!(keys.is_err());
        assert_eq!(keys.err().unwrap().to_string(), "failed to read private key file Os { code: 2, kind: NotFound, message: \"No such file or directory\" }");
    }

    #[test]
    fn test_read_invalid_private_key() {
        let path = Path::new(TESTDATA_DIR).join("invalid-private-key.pem");

        let keys = load_key(path);
        assert!(keys.is_err());
        assert_eq!(keys.err().unwrap().to_string(), "invalid private key");
    }

    #[test]
    fn test_read_valid_private_key() {
        let path = Path::new(TESTDATA_DIR).join("valid-private-key.pem");

        let keys = load_key(path);
        assert!(keys.is_ok());
    }
}
