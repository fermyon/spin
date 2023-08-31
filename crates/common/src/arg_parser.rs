//! Command line argument parsers

use anyhow::bail;

/// Parse an argument in the form `key=value` into a pair of strings.
/// The error message is specific to key-value arguments.
pub fn parse_kv(s: &str) -> anyhow::Result<(String, String)> {
    parse_eq_pair(s, "Key/Values must be of the form `key=value`")
}

fn parse_eq_pair(s: &str, err_msg: &str) -> anyhow::Result<(String, String)> {
    if let Some((key, value)) = s.split_once('=') {
        Ok((key.to_owned(), value.to_owned()))
    } else {
        bail!("{err_msg}");
    }
}
