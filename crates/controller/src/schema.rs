
// It's not really a schema

use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WorkloadId {
    id: String,
}

#[derive(Clone, Debug)]
pub struct WorkloadSpec {
    status: WorkloadStatus,
    configuration: WorkloadConfiguration,
    // TODO: how do we represent the app definition itself - by reference or by inclusion?
    // Punt for now
    manifest: WorkloadManifest,
}

#[derive(Clone, Debug)]
struct WorkloadConfiguration {
    // This is very clearly wrong but let's punt
    values: HashMap<String, String>,
}

#[derive(Clone, Debug)]
enum WorkloadStatus {
    Running,
    Stopped,
}

#[derive(Clone, Debug)]
enum WorkloadManifest {
    File(PathBuf),
    Bindle(bindle::Id),
}
