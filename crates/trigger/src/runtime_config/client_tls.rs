use anyhow::Context;
use rustls_pemfile::private_key;
use std::io;
use std::io::Cursor;
use std::{fs, path::Path};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub struct ClientTlsOpts {
    pub component_ids: Vec<String>,
    pub hosts: Vec<String>,
    pub custom_root_ca_file: Option<String>,
    pub cert_chain_file: Option<String>,
    pub private_key_file: Option<String>,
}

// load_certs parse and return the certs from the provided file
pub fn load_certs(
    path: impl AsRef<Path>,
) -> anyhow::Result<Vec<rustls_pki_types::CertificateDer<'static>>> {
    let contents = fs::read_to_string(path).expect("Should have been able to read the file");
    let mut custom_root_ca_cursor = Cursor::new(contents);

    Ok(rustls_pemfile::certs(&mut custom_root_ca_cursor)
        .into_iter()
        .map(|certs| certs.unwrap())
        .collect())
}

// load_keys parse and return the first private key from the provided file
pub fn load_keys(
    path: impl AsRef<Path>,
) -> anyhow::Result<rustls_pki_types::PrivateKeyDer<'static>> {
    private_key(&mut io::BufReader::new(
        fs::File::open(path).context("loading private key")?,
    ))
    .map_err(|_| anyhow::anyhow!("invalid input"))
    .map(|keys| keys.unwrap())
}