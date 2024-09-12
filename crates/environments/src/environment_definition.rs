use anyhow::Context;

pub async fn load_environment(env_id: &str) -> anyhow::Result<TargetEnvironment> {
    use futures_util::TryStreamExt;

    let (pkg_name, pkg_ver) = env_id.split_once('@').unwrap();

    let mut client = wasm_pkg_loader::Client::with_global_defaults()?;

    let package = pkg_name.to_owned().try_into().context("pkg ref parse")?;
    let version = wasm_pkg_loader::Version::parse(pkg_ver).context("pkg ver parse")?;

    let release = client
        .get_release(&package, &version)
        .await
        .context("get release")?;
    let stm = client
        .stream_content(&package, &release)
        .await
        .context("stream content")?;
    let bytes = stm
        .try_collect::<bytes::BytesMut>()
        .await
        .context("collect stm")?
        .to_vec();

    TargetEnvironment::new(env_id.to_owned(), bytes)
}

pub struct TargetEnvironment {
    name: String,
    decoded: wit_parser::decoding::DecodedWasm,
    package: wit_parser::Package, // saves unwrapping it every time
    package_id: id_arena::Id<wit_parser::Package>,
    package_bytes: Vec<u8>,
}

impl TargetEnvironment {
    fn new(name: String, bytes: Vec<u8>) -> anyhow::Result<Self> {
        let decoded = wit_component::decode(&bytes).context("decode wasm")?;
        let package_id = decoded.package();
        let package = decoded
            .resolve()
            .packages
            .get(package_id)
            .context("should had a package")?
            .clone();

        Ok(Self {
            name,
            decoded,
            package,
            package_id,
            package_bytes: bytes,
        })
    }

    pub fn is_world_for(&self, trigger_type: &TriggerType, world: &wit_parser::World) -> bool {
        world.name.starts_with(&format!("trigger-{trigger_type}"))
            && world.package.is_some_and(|p| p == self.package_id)
    }

    pub fn supports_trigger_type(&self, trigger_type: &TriggerType) -> bool {
        self.decoded
            .resolve()
            .worlds
            .iter()
            .any(|(_, world)| self.is_world_for(trigger_type, world))
    }

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
        let version_suffix = match self.package_version() {
            Some(version) => format!("@{version}"),
            None => "".to_owned(),
        };
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

    pub fn package_version(&self) -> Option<&semver::Version> {
        self.package.name.version.as_ref()
    }

    pub fn package_bytes(&self) -> &[u8] {
        &self.package_bytes
    }
}

pub type TriggerType = String;
