#[derive(Default)]
pub struct DeploymentTargets {
    target_environments: Vec<DeploymentTarget>,
}
pub type DeploymentTarget = String;

impl DeploymentTargets {
    pub fn new(envs: Vec<String>) -> Self {
        Self {
            target_environments: envs,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.target_environments.iter().map(|s| s.as_str())
    }

    pub fn is_empty(&self) -> bool {
        // TODO: it would be nice to let "no-op" behaviour fall out organically,
        // but currently we do some stuff eagerly, so...
        self.target_environments.is_empty()
    }
}
