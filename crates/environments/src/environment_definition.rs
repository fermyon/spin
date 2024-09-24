use std::path::Path;

use anyhow::Context;
use spin_common::ui::quoted_path;
use spin_manifest::schema::v2::TargetEnvironmentRef;

const DEFAULT_REGISTRY: &str = "fermyon.com";

/// Loads the given `TargetEnvironment` from a registry.
pub async fn load_environment(env_id: &TargetEnvironmentRef) -> anyhow::Result<TargetEnvironment> {
    match env_id {
        TargetEnvironmentRef::DefaultRegistry(package) => {
            load_environment_from_registry(DEFAULT_REGISTRY, package).await
        }
        TargetEnvironmentRef::Registry { registry, package } => {
            load_environment_from_registry(registry, package).await
        }
        TargetEnvironmentRef::WitDirectory { path } => load_environment_from_dir(path),
    }
}

async fn load_environment_from_registry(
    registry: &str,
    env_id: &str,
) -> anyhow::Result<TargetEnvironment> {
    use futures_util::TryStreamExt;

    let (pkg_name, pkg_ver) = env_id.split_once('@').with_context(|| format!("Failed to parse target environment {env_id} as package reference - is the target correct?"))?;
    let env_pkg_ref: wasm_pkg_loader::PackageRef = pkg_name
        .parse()
        .with_context(|| format!("Environment {pkg_name} is not a valid package name"))?;

    let registry: wasm_pkg_loader::Registry = registry
        .parse()
        .with_context(|| format!("Registry {registry} is not a valid registry name"))?;

    // TODO: this requires wkg configuration which shouldn't be on users:
    // is there a better way to handle it?
    let mut wkg_config = wasm_pkg_loader::Config::global_defaults()
        .unwrap_or_else(|_| wasm_pkg_loader::Config::empty());
    wkg_config.set_package_registry_override(env_pkg_ref, registry);

    let mut client = wasm_pkg_loader::Client::new(wkg_config);

    let package = pkg_name
        .to_owned()
        .try_into()
        .with_context(|| format!("Failed to parse environment name {pkg_name} as package name"))?;
    let version = wasm_pkg_loader::Version::parse(pkg_ver).with_context(|| {
        format!("Failed to parse environment version {pkg_ver} as package version")
    })?;

    let release = client
        .get_release(&package, &version)
        .await
        .with_context(|| format!("Failed to get {env_id} release from registry"))?;
    let stm = client
        .stream_content(&package, &release)
        .await
        .with_context(|| format!("Failed to get {env_id} package from registry"))?;
    let bytes = stm
        .try_collect::<bytes::BytesMut>()
        .await
        .with_context(|| format!("Failed to get {env_id} package data from registry"))?
        .to_vec();

    TargetEnvironment::from_package_bytes(env_id.to_owned(), bytes)
}

fn load_environment_from_dir(path: &Path) -> anyhow::Result<TargetEnvironment> {
    let mut resolve = wit_parser::Resolve::default();
    let (pkg_id, _) = resolve.push_dir(path)?;
    let decoded = wit_parser::decoding::DecodedWasm::WitPackage(resolve, pkg_id);
    TargetEnvironment::from_decoded_wasm(path, decoded)
}

/// A parsed document representing a deployment environment, e.g. Spin 2.7,
/// SpinKube 3.1, Fermyon Cloud. The `TargetEnvironment` provides a mapping
/// from the Spin trigger types supported in the environment to the Component Model worlds
/// supported by that trigger type. (A trigger type may support more than one world,
/// for example when it supports multiple versions of the Spin or WASI interfaces.)
///
/// In terms of implementation, internally the environment is represented by a
/// WIT package that adheres to a specific naming convention (that the worlds for
/// a given trigger type are exactly whose names begin `trigger-xxx` where
/// `xxx` is the Spin trigger type).
pub struct TargetEnvironment {
    name: String,
    decoded: wit_parser::decoding::DecodedWasm,
    package: wit_parser::Package,
    package_id: id_arena::Id<wit_parser::Package>,
    package_bytes: Vec<u8>,
}

impl TargetEnvironment {
    fn from_package_bytes(name: String, bytes: Vec<u8>) -> anyhow::Result<Self> {
        let decoded = wit_component::decode(&bytes)
            .with_context(|| format!("Failed to decode package for environment {name}"))?;
        let package_id = decoded.package();
        let package = decoded
            .resolve()
            .packages
            .get(package_id)
            .with_context(|| {
                format!("The {name} environment is invalid (no package for decoded package ID)")
            })?
            .clone();

        Ok(Self {
            name,
            decoded,
            package,
            package_id,
            package_bytes: bytes,
        })
    }

    fn from_decoded_wasm(
        source: &Path,
        decoded: wit_parser::decoding::DecodedWasm,
    ) -> anyhow::Result<Self> {
        let package_id = decoded.package();
        let package = decoded
            .resolve()
            .packages
            .get(package_id)
            .with_context(|| {
                format!(
                    "The {} environment is invalid (no package for decoded package ID)",
                    quoted_path(source)
                )
            })?
            .clone();
        let name = package.name.to_string();

        // This versionm of wit_component requires a flag for v2 encoding.
        // v1 encoding is retired in wit_component main. You can remove the
        // flag when this breaks next time we upgrade the crate!
        let bytes = wit_component::encode(Some(true), decoded.resolve(), package_id)?;

        Ok(Self {
            name,
            decoded,
            package,
            package_id,
            package_bytes: bytes,
        })
    }

    /// Returns true if the given trigger type provides the world identified by
    /// `world` in this environment.
    pub fn is_world_for(&self, trigger_type: &TriggerType, world: &wit_parser::World) -> bool {
        world.name.starts_with(&format!("trigger-{trigger_type}"))
            && world.package.is_some_and(|p| p == self.package_id)
    }

    /// Returns true if the given trigger type can run in this environment.
    pub fn supports_trigger_type(&self, trigger_type: &TriggerType) -> bool {
        self.decoded
            .resolve()
            .worlds
            .iter()
            .any(|(_, world)| self.is_world_for(trigger_type, world))
    }

    /// Lists all worlds supported for the given trigger type in this environment.
    pub fn worlds(&self, trigger_type: &TriggerType) -> Vec<String> {
        self.decoded
            .resolve()
            .worlds
            .iter()
            .filter(|(_, world)| self.is_world_for(trigger_type, world))
            .map(|(_, world)| self.world_qname(world))
            .collect()
    }

    /// Fully qualified world name (e.g. fermyon:spin/http-trigger@2.0.0)
    fn world_qname(&self, world: &wit_parser::World) -> String {
        let version_suffix = self
            .package_version()
            .map(|version| format!("@{version}"))
            .unwrap_or_default();
        format!(
            "{}/{}{version_suffix}",
            self.package_namespaced_name(),
            world.name,
        )
    }

    /// The environment name for UI purposes
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Namespaced but unversioned package name (e.g. spin:cli)
    pub fn package_namespaced_name(&self) -> String {
        format!("{}:{}", self.package.name.namespace, self.package.name.name)
    }

    /// The package version for the environment package.
    pub fn package_version(&self) -> Option<&semver::Version> {
        self.package.name.version.as_ref()
    }

    /// The Wasm-encoded bytes of the environment package.
    pub fn package_bytes(&self) -> &[u8] {
        &self.package_bytes
    }
}

pub type TriggerType = String;
