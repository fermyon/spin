use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginManifest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    homepage: Option<String>,
    pub version: String,
    pub spin_compatibility: String,
    pub license: String,
    pub packages: Vec<PluginPackage>,
}

#[derive(Serialize, Debug, Deserialize, PartialEq)]
pub(crate) struct PluginPackage {
    pub os: Os,
    pub arch: Architecture,
    pub url: String,
    pub sha256: String,
}

#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Os {
    Linux,
    Osx,
    Windows,
}

#[derive(Serialize, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Architecture {
    Amd64,
    Aarch64,
}

// TODO: create licenses enum

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
