use std::{collections::HashMap, sync::{Arc, RwLock}, path::{Path, PathBuf}};

use anyhow::{bail, Context};
use tempfile::TempDir;
use tokio::task::JoinHandle;

use crate::{schema::{WorkloadId, WorkloadOperation}, store::WorkStore, WorkloadSpec, WorkloadEvent, run::run, WorkloadStatus};

#[async_trait::async_trait]
pub(crate) trait Scheduler {
    async fn notify_changed(&self, workload: &WorkloadId) -> anyhow::Result<()>;
}

pub(crate) struct LocalScheduler {
    store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
    running: Arc<RwLock<HashMap<WorkloadId, RunningWorkload>>>,
    event_sender: tokio::sync::broadcast::Sender<WorkloadEvent>,
    operation_receiver: tokio::sync::broadcast::Receiver<WorkloadOperation>,
}

impl LocalScheduler {
    pub(crate) fn new(
        store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
        event_sender: &tokio::sync::broadcast::Sender<WorkloadEvent>,
        operation_receiver: tokio::sync::broadcast::Receiver<WorkloadOperation>
    ) -> Self {
        Self {
            store,
            running: Arc::new(RwLock::new(HashMap::new())),
            event_sender: event_sender.clone(),
            operation_receiver,
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
    pub fn start(self) -> JoinHandle<()> {
        tokio::task::spawn(
            self.run_event_loop()
        )
    }

    async fn run_event_loop(mut self) {
        loop {
            match self.operation_receiver.recv().await {
                Ok(oper) => {
                    self.process_operation(oper).await;
                },
                Err(_) => {
                    println!("SCHED: Oh no!");
                    break;
                }
            }
        }
    }

    async fn process_operation(&self, oper: WorkloadOperation) {
        let evt = match oper {
            WorkloadOperation::Changed(workload) =>
                self.process_workload_changed(&workload).await.err()
                    .map(|e| WorkloadEvent::UpdateFailed(workload.clone(), Arc::new(e))),
        };

        match evt {
            None => (),
            Some(evt) => {
                match self.event_sender.send(evt) {
                    Ok(_) => (),
                    Err(_) => {
                        println!("SCHED: process_operation error, and send failed");
                    },
                }
            }
        }
    }

    async fn process_workload_changed(&self, workload: &WorkloadId) -> anyhow::Result<()> {
        // TODO: look at WorkloadSpec::status
        match self.extricate(workload) {
            (Some(w), Some(c)) => {
                if w.status == WorkloadStatus::Running {
                    self.restart_workload(workload, w, c).await?
                } else {
                    self.stop_workload(workload, c)
                }
            },
            (Some(w), None) => {
                if w.status == WorkloadStatus::Running {
                    self.start_workload(workload, w).await?
                } else {
                    ()
                }
            },
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
        let running = run(workload, spec, &self.event_sender).await?;
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
