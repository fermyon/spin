use std::{collections::HashMap, sync::{Arc, RwLock}, path::{Path, PathBuf}};

use anyhow::{bail, Context};
use tempfile::TempDir;

use crate::{schema::WorkloadId, store::WorkStore, WorkloadSpec, WorkloadEvent, run::run};

#[async_trait::async_trait]
pub(crate) trait Scheduler {
    async fn notify_changed(&self, workload: &WorkloadId) -> anyhow::Result<()>;
}

pub(crate) struct LocalScheduler {
    store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
    running: Arc<RwLock<HashMap<WorkloadId, RunningWorkload>>>,
    notification_sender: crossbeam_channel::Sender<WorkloadEvent>,
}

impl LocalScheduler {
    pub(crate) fn new(store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>, notification_sender: &crossbeam_channel::Sender<WorkloadEvent>) -> Self {
        Self {
            store,
            running: Arc::new(RwLock::new(HashMap::new())),
            notification_sender: notification_sender.clone(),
        }
    }
}

pub(crate) struct RunningWorkload {
    pub(crate) work_dir: WorkingDirectory,
    pub(crate) handle: RunHandle,
}

pub(crate) enum RunHandle {
    Fut(tokio::task::JoinHandle<()>),
}

impl LocalScheduler {
    pub async fn notify_changed(&self, workload: &WorkloadId) -> anyhow::Result<()> {
        // TODO: look at WorkloadSpec::status
        match self.extricate(workload) {
            (Some(w), Some(c)) => self.restart_workload(workload, w, c).await?,
            (Some(w), None) => self.start_workload(workload, w).await?,
            (None, Some(c)) => self.stop_workload(workload, c),
            (None, None) => (),
        }

        Ok(())
    }

    fn extricate(&self, workload: &WorkloadId) -> (Option<WorkloadSpec>, Option<RunningWorkload>) {
        let spec = self.store.read().unwrap().get_workload(workload);
        let mut running = self.running.write().unwrap();
        let current = running.remove(workload);
        (spec, current)
    }

    async fn start_workload(&self, workload: &WorkloadId, spec: WorkloadSpec) -> anyhow::Result<()> {
        // Identify the application type
        // Instantiate the relevant trigger
        // Start the relevant trigger
        let running = run(workload, spec, &self.notification_sender).await?;
        // Stash the task
        self.running.write().unwrap().insert(workload.clone(), running);
        Ok(())
    }

    async fn restart_workload(&self, workload: &WorkloadId, spec: WorkloadSpec, current: RunningWorkload) -> anyhow::Result<()> {
        self.stop_workload(workload, current);
        self.start_workload(workload, spec).await
    }

    fn stop_workload(&self, workload: &WorkloadId, current: RunningWorkload) {
        current.stop();
        self.running.write().unwrap().remove(workload);
    }
}

impl RunningWorkload {
    fn stop(self) {
        match self.handle {
            RunHandle::Fut(f) => f.abort(),
        }
        drop(self.work_dir);
    }
}

pub(crate) enum WorkingDirectory {
    Given(PathBuf),
    Temporary(TempDir),
}

impl WorkingDirectory {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Given(p) => p,
            Self::Temporary(t) => t.path(),
        }
    }
}
