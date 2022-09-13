use crate::manifest::{PluginManifest, PluginPackage};
use anyhow::Result;
use std::io;

fn are_you_sure() -> Result<bool> {
    let mut resp = String::new();
    io::stdin().read_line(&mut resp)?;
    Ok(parse_response(&mut resp))
}

fn parse_response(resp: &mut str) -> bool {
    let resp = resp.trim().to_lowercase();
    resp == "yes" || resp == "y"
}

/// Prompts user as to whether they trust the source of the plugin and
/// want to proceed with installation. Returns whether to proceed.
pub fn prompt_confirm_install(manifest: &PluginManifest, package: &PluginPackage) -> Result<bool> {
    println!(
        "Installing plugin {} with license {} from {}\n",
        &manifest.name(),
        &manifest.license,
        &package.url
    );
    println!("Are you sure you want to proceed? (y/N)");
    let sure = are_you_sure()?;
    if !sure {
        println!("Plugin {} will not be installed", manifest.name());
    }
    Ok(sure)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_response() {
        let input_output: Vec<(String, bool)> = vec![
            ("\n\t  yes   ".to_string(), true),
            ("YES".to_string(), true),
            ("y".to_string(), true),
            ("Y".to_string(), true),
            ("  no".to_string(), false),
            ("n".to_string(), false),
            ("N".to_string(), false),
            ("random".to_string(), false),
        ];
        for (mut i, o) in input_output {
            assert_eq!(parse_response(&mut i), o);
        }
    }
}
