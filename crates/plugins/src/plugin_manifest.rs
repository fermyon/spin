use serde::{Deserialize, Serialize};

/// Expected schema of a plugin manifest. Should match the latest Spin plugin
/// manifest JSON schema:
/// https://github.com/fermyon/spin-plugins/tree/main/json-schema
#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginManifest {
    /// Name of the plugin.
    name: String,
    /// Option description of the plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Optional address to the homepage of the plugin producer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    homepage: Option<String>,
    /// Version of the plugin.
    pub version: String,
    /// Versions of Spin that the plugin is compatible with.
    pub spin_compatibility: String,
    /// License of the plugin.
    pub license: String,
    /// Points to source package[s] of the plugin..
    pub packages: Vec<PluginPackage>,
}

impl PluginManifest {
    pub fn name(&self) -> String {
        self.name.to_lowercase()
    }
}

/// Describes compatibility and location of a plugin source.
#[derive(Serialize, Debug, Deserialize, PartialEq)]
pub(crate) struct PluginPackage {
    /// Compatible OS.
    pub os: Os,
    /// Compatible architecture.
    pub arch: Architecture,
    /// Address to fetch the plugin source tar file.
    pub url: String,
    /// Checksum to verify the plugin before installation.
    pub sha256: String,
}

/// Describes the compatible OS of a plugin
#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Os {
    Linux,
    Osx,
    Windows,
}

/// Describes the compatible architecture of a plugin
#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Architecture {
    Amd64,
    Aarch64,
    Arm,
}

impl ToString for Architecture {
    fn to_string(&self) -> String {
        match self {
            Self::Amd64 => "x86_64".to_string(),
            Self::Aarch64 => "aarch64".to_string(),
            Self::Arm => "arm".to_string(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_plugin_json() {
        let name = "test";
        let description = "Some description.";
        let homepage = "www.example.com";
        let version = "1.0";
        let license = "Mit";
        let plugin_json = r#"
        {
            "name": "test",
            "description": "Some description.",
            "homepage": "www.example.com",
            "version": "1.0",
            "spinCompatibility": "=0.4",
            "license": "Mit",
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
                    "os": "osx",
                    "arch": "aarch64",
                    "url": "www.example.com/releases/1.0/binary.tgz",
                    "sha256": "eeegf00b12345e38acae2d19b2a707a4fhdjdfdd22875efeefdf052ce19c90b"
                }
            ]
        }"#;

        let deserialized_plugin: PluginManifest = serde_json::from_str(plugin_json).unwrap();
        assert_eq!(deserialized_plugin.name, name.to_owned());
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
        let name = "test";
        let version = "1.0";
        let license = "Mit";
        let plugin_json = r#"
        {
            "name": "test",
            "version": "1.0",
            "spinCompatibility": "=0.4",
            "license": "Mit",
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
                    "os": "osx",
                    "arch": "aarch64",
                    "url": "www.example.com/releases/1.0/binary.tgz",
                    "sha256": "eeegf00b12345e38acae2d19b2a707a4fhdjdfdd22875efeefdf052ce19c90b"
                }
            ]
        }"#;

        let deserialized_plugin: PluginManifest = serde_json::from_str(plugin_json).unwrap();
        assert_eq!(deserialized_plugin.name, name.to_owned());
        assert_eq!(deserialized_plugin.description, None);
        assert_eq!(deserialized_plugin.homepage, None);
        assert_eq!(deserialized_plugin.version, version.to_owned());
        assert_eq!(deserialized_plugin.license, license.to_owned());
        assert_eq!(deserialized_plugin.packages.len(), 3);
    }
}
