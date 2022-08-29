use crate::{
    get_manifest_file_name, plugin_manifest::PluginManifest, PLUGIN_MANIFESTS_DIRECTORY_NAME,
};
use anyhow::{anyhow, Result};
use semver::{Version, VersionReq};
use std::{fs::File, path::Path};

/// Checks whether the plugin supports the currently running version of Spin.
// TODO: check whether on main or canary (aka beyond the specified version).
pub fn assert_supported_version(spin_version: &str, supported: &str) -> Result<()> {
    let supported = VersionReq::parse(supported).map_err(|e| {
        anyhow!(
            "could not parse manifest compatibility version {} as valid semver -- {:?}",
            supported,
            e
        )
    })?;
    let version = Version::parse(spin_version)?;
    match supported.matches(&version) {
        true => Ok(()),
        false => Err(anyhow!(
            "plugin is compatible with Spin {} but running Spin {}",
            supported,
            spin_version
        )),
    }
}

pub(crate) fn get_plugin_manifest(plugin_name: &str, plugins_dir: &Path) -> Result<PluginManifest> {
    let manifest_path = plugins_dir
        .join(PLUGIN_MANIFESTS_DIRECTORY_NAME)
        .join(get_manifest_file_name(plugin_name));
    log::info!("Reading plugin manifest from {:?}", manifest_path);
    let manifest_file = File::open(manifest_path)?;
    let manifest = serde_json::from_reader(manifest_file)?;
    Ok(manifest)
}

pub fn check_plugin_spin_compatibility(
    plugin_name: &str,
    spin_version: &str,
    plugins_dir: &Path,
) -> Result<()> {
    let manifest = get_plugin_manifest(plugin_name, plugins_dir)?;
    assert_supported_version(spin_version, &manifest.spin_compatibility)
}

#[cfg(test)]
mod version_tests {
    use super::*;
    #[test]
    fn test_supported_version() {
        let test_case = ">=1.2.3, <1.8.0";
        let input_output = [
            ("1.3.0", true),
            ("1.2.3", true),
            ("1.8.0", false),
            ("1.9.0", false),
            ("1.2.0", false),
        ];
        input_output
            .into_iter()
            .for_each(|(i, o)| assert_eq!(assert_supported_version(i, test_case).is_err(), !o));
    }
}
