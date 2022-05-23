use std::sync::{RwLock, Arc};

use scheduler::{Scheduler, LocalScheduler};
use schema::WorkloadOperation;
pub use schema::{WorkloadEvent, WorkloadId, WorkloadManifest, WorkloadOpts, WorkloadSpec, WorkloadStatus};
use store::{WorkStore, InMemoryWorkStore};

mod run;
pub(crate) mod scheduler;
pub(crate) mod schema;
pub(crate) mod store;

pub struct Control {
    _scheduler: tokio::task::JoinHandle<()>,
    store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
    event_sender: tokio::sync::broadcast::Sender<WorkloadEvent>,  // For in memory it sorta works to have the comms directly from scheduler but WHO KNOWS
    _event_receiver: tokio::sync::broadcast::Receiver<WorkloadEvent>,
    scheduler_notifier: tokio::sync::broadcast::Sender<WorkloadOperation>,
}

impl Control {
    pub fn in_memory() -> Self {
        let box_store: Box<dyn WorkStore + Send + Sync> = Box::new(InMemoryWorkStore::new());
        let store = Arc::new(RwLock::new(box_store));
        let (evt_tx, evt_rx) = tokio::sync::broadcast::channel(1000);
        let (oper_tx, oper_rx) = tokio::sync::broadcast::channel(1000);
        let scheduler = LocalScheduler::new(store.clone(), &evt_tx, oper_rx);
        let jh = tokio::task::spawn(scheduler.start());
        Self {
            _scheduler: jh,
            store,
            event_sender: evt_tx,
            _event_receiver: evt_rx,
            scheduler_notifier: oper_tx,
        }
    }

    pub async fn set_workload(&mut self, workload: &WorkloadId, spec: WorkloadSpec) -> anyhow::Result<()> {
        self.store.write().unwrap().set_workload(workload, spec);
        let oper = WorkloadOperation::Changed(workload.clone());
        self.scheduler_notifier.send(oper)?;
        Ok(())
    }

    pub async fn remove_workload(&mut self, workload: &WorkloadId) -> anyhow::Result<()> {
        self.store.write().unwrap().remove_workload(workload);
        let oper = WorkloadOperation::Changed(workload.clone());
        self.scheduler_notifier.send(oper)?;
        Ok(())
    }

    pub fn notifications(&self) -> tokio::sync::broadcast::Receiver<WorkloadEvent> {
        self.event_sender.subscribe()
    }
}
