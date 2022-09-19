use anyhow::{anyhow, Context, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

/// Expected schema of a plugin manifest. Should match the latest Spin plugin
/// manifest JSON schema:
/// https://github.com/fermyon/spin-plugins/tree/main/json-schema
#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    /// Name of the plugin.
    name: String,
    /// Option description of the plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Optional address to the homepage of the plugin producer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    homepage: Option<String>,
    /// Version of the plugin.
    pub(crate) version: String,
    /// Versions of Spin that the plugin is compatible with.
    pub(crate) spin_compatibility: String,
    /// License of the plugin.
    pub(crate) license: String,
    /// Points to source package[s] of the plugin..
    pub(crate) packages: Vec<PluginPackage>,
}

impl PluginManifest {
    pub fn name(&self) -> String {
        self.name.to_lowercase()
    }
}

/// Describes compatibility and location of a plugin source.
#[derive(Serialize, Debug, Deserialize, PartialEq)]
pub struct PluginPackage {
    /// Compatible OS.
    pub(crate) os: Os,
    /// Compatible architecture.
    pub(crate) arch: Architecture,
    /// Address to fetch the plugin source tar file.
    pub(crate) url: String,
    /// Checksum to verify the plugin before installation.
    pub(crate) sha256: String,
}

/// Describes the compatible OS of a plugin
#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Os {
    Linux,
    Macos,
    Windows,
}

impl Os {
    // Maps manifest OS options to associated Rust OS strings
    // https://doc.rust-lang.org/std/env/consts/constant.OS.html
    pub(crate) fn rust_name(&self) -> &'static str {
        match self {
            Os::Linux => "linux",
            Os::Macos => "macos",
            Os::Windows => "windows",
        }
    }
}

/// Describes the compatible architecture of a plugin
#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Architecture {
    Amd64,
    Aarch64,
    Arm,
}

impl Architecture {
    // Maps manifest Architecture options to associated Rust ARCH strings
    // https://doc.rust-lang.org/std/env/consts/constant.ARCH.html
    pub(crate) fn rust_name(&self) -> &'static str {
        match self {
            Architecture::Amd64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
            Architecture::Arm => "arm",
        }
    }
}

/// Checks whether the plugin supports the currently running version of Spin.
pub fn check_supported_version(manifest: &PluginManifest, spin_version: &str) -> Result<()> {
    let supported_on = &manifest.spin_compatibility;
    inner_check_supported_version(supported_on, spin_version)
}

fn inner_check_supported_version(supported_on: &str, spin_version: &str) -> Result<()> {
    let comparator = VersionReq::parse(supported_on).with_context(|| {
        format!(
            "Could not parse manifest compatibility version {} as valid semver",
            &supported_on,
        )
    })?;
    let version = Version::parse(spin_version)?;
    if !comparator.matches(&version) {
        return Err(anyhow!(
            "Spin version compatibility check failed (supported: {supported_on}, actual: {spin_version}). Try running `spin plugin update` to get latest."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn generate_test_manifest(
        name: &str,
        version: &str,
        license: &str,
        description: Option<&str>,
        homepage: Option<&str>,
    ) -> PluginManifest {
        let mut plugin_json = serde_json::json!(
        {
            "name": name,
            "version": version,
            "spinCompatibility": "=0.4",
            "license": license,
            "packages": [
                {
                    "os": "linux",
                    "arch": "amd64",
                    "url": "www.example.com/releases/1.0/binary.tgz",
                    "sha256": "c474f00b12345e38acae2d19b2a707a4fhdjdfdd22875efeefdf052ce19c90b"
                },
                {
                    "os": "windows",
                    "arch": "amd64",
                    "url": "www.example.com/releases/1.0/binary.tgz",
                    "sha256": "eee4f00b12345e38acae2d19b2a707a4fhdjdfdd22875efeefdf052ce19c90b"
                },
                {
                    "os": "macos",
                    "arch": "aarch64",
                    "url": "www.example.com/releases/1.0/binary.tgz",
                    "sha256": "eeegf00b12345e38acae2d19b2a707a4fhdjdfdd22875efeefdf052ce19c90b"
                }
            ]
        });
        if let Some(homepage) = homepage {
            plugin_json
                .as_object_mut()
                .unwrap()
                .insert("homepage".to_string(), serde_json::json!(homepage));
        }
        if let Some(description) = description {
            plugin_json
                .as_object_mut()
                .unwrap()
                .insert("description".to_string(), serde_json::json!(description));
        }
        serde_json::from_value(plugin_json).unwrap()
    }

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
            .for_each(|(i, o)| assert_eq!(inner_check_supported_version(test_case, i).is_ok(), o));
    }

    #[test]
    fn test_plugin_json() {
        let name = "test";
        let description = "Some description.";
        let homepage = "www.example.com";
        let version = "1.0";
        let license = "Mit";
        let deserialized_plugin =
            generate_test_manifest(name, version, license, Some(description), Some(homepage));
        assert_eq!(deserialized_plugin.name(), name.to_owned());
        assert_eq!(
            deserialized_plugin.description,
            Some(description.to_owned())
        );
        assert_eq!(deserialized_plugin.homepage, Some(homepage.to_owned()));
        assert_eq!(deserialized_plugin.version, version.to_owned());
        assert_eq!(deserialized_plugin.license, license.to_owned());
        assert_eq!(deserialized_plugin.packages.len(), 3);
    }

    #[test]
    fn test_plugin_json_empty_options() {
        let deserialized_plugin = generate_test_manifest("name", "0.1.1", "Mit", None, None);
        assert_eq!(deserialized_plugin.description, None);
        assert_eq!(deserialized_plugin.homepage, None);
    }
}
