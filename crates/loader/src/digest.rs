//! Various digest functions.

use sha2::{Digest, Sha256};
use std::path::Path;

/// Return the hex-encoded SHA256 digest of the given bytes.
pub fn bytes_sha256_string(bytes: &[u8]) -> String {
    let digest_value = Sha256::digest(bytes);

    to_hex_string(digest_value)
}

/// Return the hex-encoded SHA256 digest of the given file.
pub fn file_sha256_string(path: impl AsRef<Path>) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut sha = sha2::Sha256::new();
    std::io::copy(&mut file, &mut sha)?;
    let digest_value = sha.finalize();

    Ok(to_hex_string(digest_value))
}

fn to_hex_string(digest_value: impl std::fmt::LowerHex) -> String {
    format!("{:x}", digest_value)
}
