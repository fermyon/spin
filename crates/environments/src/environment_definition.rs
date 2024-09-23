use anyhow::Context;

/// Loads the given `TargetEnvironment` from a registry.
pub async fn load_environment(env_id: impl AsRef<str>) -> anyhow::Result<TargetEnvironment> {
    use futures_util::TryStreamExt;

    let env_id = env_id.as_ref();

    let (pkg_name, pkg_ver) = env_id.split_once('@').with_context(|| format!("Failed to parse target environment {env_id} as package reference - is the target correct?"))?;

    // TODO: this requires wkg configuration which shouldn't be on users:
    // is there a better way to handle it?
    let mut client = wasm_pkg_loader::Client::with_global_defaults()
        .context("Failed to create a package loader from your global settings")?;

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

    TargetEnvironment::new(env_id.to_owned(), bytes)
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
    fn new(name: String, bytes: Vec<u8>) -> anyhow::Result<Self> {
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
