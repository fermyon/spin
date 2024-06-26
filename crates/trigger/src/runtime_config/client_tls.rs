use anyhow::Context;
use rustls_pemfile::private_key;
use std::io;
use std::io::Cursor;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub struct ClientTlsOpts {
    pub component_ids: Vec<spin_serde::KebabId>,
    pub hosts: Vec<String>,
    pub ca_roots_file: Option<PathBuf>,
    pub cert_chain_file: Option<PathBuf>,
    pub private_key_file: Option<PathBuf>,
    pub ca_webpki_roots: Option<bool>,
}

// load_certs parse and return the certs from the provided file
pub fn load_certs(
    path: impl AsRef<Path>,
) -> anyhow::Result<Vec<rustls_pki_types::CertificateDer<'static>>> {
    let contents = fs::read_to_string(path).expect("Should have been able to read the file");
    let mut custom_root_ca_cursor = Cursor::new(contents);

    Ok(rustls_pemfile::certs(&mut custom_root_ca_cursor)
        .map(|certs| certs.unwrap())
        .collect())
}

// load_keys parse and return the first private key from the provided file
pub fn load_key(
    path: impl AsRef<Path>,
) -> anyhow::Result<rustls_pki_types::PrivateKeyDer<'static>> {
    private_key(&mut io::BufReader::new(
        fs::File::open(path).context("loading private key")?,
    ))
    .map_err(|_| anyhow::anyhow!("invalid input"))
    .map(|keys| keys.unwrap())
}
