use wasm_pkg_loader::PackageRef;

#[derive(Debug, serde::Deserialize)]
pub struct TargetEnvironment {
    pub name: String,
    pub environments: std::collections::HashMap<TriggerType, TargetWorld>,
}

#[derive(Debug, Eq, Hash, PartialEq, serde::Deserialize)]
pub struct TargetWorld {
    wit_package: PackageRef,
    package_ver: String, // TODO: tidy to semver::Version
    world_name: WorldNames,
}

#[derive(Debug, Eq, Hash, PartialEq, serde::Deserialize)]
#[serde(untagged)]
enum WorldNames {
    Exactly(String),
    AnyOf(Vec<String>),
}

impl TargetWorld {
    fn versioned_name(&self, world_name: &str) -> String {
        format!("{}/{}@{}", self.wit_package, world_name, self.package_ver)
    }

    pub fn versioned_names(&self) -> Vec<String> {
        match &self.world_name {
            WorldNames::Exactly(name) => vec![self.versioned_name(name)],
            WorldNames::AnyOf(names) => {
                names.iter().map(|name| self.versioned_name(name)).collect()
            }
        }
    }
}

pub type TriggerType = String;
