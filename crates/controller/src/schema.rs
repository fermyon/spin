
// It's not really a schema

use std::{path::PathBuf};

use spin_engine::io::FollowComponents;
use spin_loader::bindle::BindleConnectionInfo;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WorkloadId {
    id: String,
}

impl WorkloadId {
    pub fn new() -> Self {
        let id = format!("{}", uuid::Uuid::new_v4().hyphenated());
        Self { id }
    }
}

#[derive(Clone, Debug)]
pub struct WorkloadSpec {
    pub status: WorkloadStatus,
    pub opts: WorkloadOpts,
    // TODO: how do we represent the app definition itself - by reference or by inclusion?
    // Punt for now
    pub manifest: WorkloadManifest,
}

// #[derive(Clone, Debug)]
// struct WorkloadConfiguration {
//     // This is very clearly wrong but let's punt
//     values: HashMap<String, String>,
// }

#[derive(Clone, Debug)]
pub enum WorkloadStatus {
    Running,
    Stopped,
}

#[derive(Clone, Debug)]
pub enum WorkloadManifest {
    File(PathBuf),
    Bindle(String /*bindle::Id*/),
}

// UpOpts in a trenchcoat
#[derive(Clone, Debug)]
pub struct WorkloadOpts {
    pub server: Option<String>,
    pub address: String,
    pub tmp: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub log: Option<PathBuf>,
    pub disable_cache: bool,
    pub cache: Option<PathBuf>,
    pub follow_components: Vec<String>,
    pub follow_all_components: bool,
}

impl WorkloadOpts {
    pub(crate) fn wasmtime_default_config(&self) -> anyhow::Result<wasmtime::Config> {
        let mut wasmtime_config = wasmtime::Config::default();
        if !self.disable_cache {
            match &self.cache {
                Some(p) => wasmtime_config.cache_config_load(p)?,
                None => wasmtime_config.cache_config_load_default()?,
            };
        }
        Ok(wasmtime_config)
    }

    pub(crate) fn follow_components(&self) -> FollowComponents {
        if self.follow_all_components {
            FollowComponents::All
        } else if self.follow_components.is_empty() {
            FollowComponents::None
        } else {
            let followed = self.follow_components.clone().into_iter().collect();
            FollowComponents::Named(followed)
        }
    }

    pub(crate) fn bindle_connection(&self) -> Option<BindleConnectionInfo> {
        self.server
            .as_ref()
            .map(|url| BindleConnectionInfo::new(url, false, None, None))
    }
}
