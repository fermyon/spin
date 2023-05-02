//! SHA-256 digest

use std::path::Path;

use sha2::{Digest, Sha256};

/// Return the hex SHA-256 digest of the given bytes.
pub fn hex_digest_from_bytes(bytes: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

/// Return the hex SHA-256 digest of the given file.
pub fn hex_digest_from_file(path: impl AsRef<Path>) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let digest = hasher.finalize();
    Ok(format!("{digest:x}"))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn test_hex_digest_from_bytes() {
        let hex = hex_digest_from_bytes("spin");
        assert_eq!(
            hex,
            "a5a2729ffa0eeacc15323a9168807c72d18d1cb375dbde899c44d6803dad2b19"
        );
    }

    #[test]
    fn test_hex_digest_from_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(&mut f, "spin").unwrap();
        let hex = hex_digest_from_file(f.into_temp_path()).unwrap();
        assert_eq!(
            hex,
            "a5a2729ffa0eeacc15323a9168807c72d18d1cb375dbde899c44d6803dad2b19"
        );
    }
}
