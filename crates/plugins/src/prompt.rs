use crate::manifest::{PluginManifest, PluginPackage};
use anyhow::Result;
use std::io;

/// Prompts user as to whether they trust the source of the plugin and
/// want to proceed with installation.
fn print_prompt(name: &str, license: &str, source_url: &str) {
    println!(
        "Installing plugin {} with license {} from {}\n",
        name, license, source_url
    );
    println!("Are you sure you want to proceed? (y/N)");
}

fn are_you_sure() -> Result<bool> {
    let mut resp = String::new();
    io::stdin().read_line(&mut resp)?;
    Ok(parse_response(&mut resp))
}

fn parse_response(resp: &mut str) -> bool {
    let resp = resp.trim().to_lowercase();
    resp == "yes" || resp == "y"
}

/// Returns whether or not the user would like to proceed with the installation of a plugin.
pub fn prompt(manifest: &PluginManifest, package: &PluginPackage) -> Result<bool> {
    print_prompt(&manifest.name(), &manifest.license, &package.url);
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
        let mut resp = String::from("\n\t  yes   ");
        assert!(parse_response(&mut resp));
    }
}
