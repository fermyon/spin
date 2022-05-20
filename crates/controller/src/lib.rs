use std::sync::{RwLock, Arc};

use scheduler::{Scheduler, LocalScheduler};
pub use schema::{WorkloadId, WorkloadManifest, WorkloadOpts, WorkloadSpec, WorkloadStatus};
use store::{WorkStore, InMemoryWorkStore};

pub(crate) mod scheduler;
pub(crate) mod schema;
pub(crate) mod store;

pub struct Control {
    scheduler: LocalScheduler,  // having some grief with the async trait stuff on the Scheduler trait
    store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
}

impl Control {
    pub fn in_memory() -> Self {
        let box_store: Box<dyn WorkStore + Send + Sync> = Box::new(InMemoryWorkStore::new());
        let store = Arc::new(RwLock::new(box_store));
        Self {
            scheduler: LocalScheduler::new(store.clone()),
            store,
        }
    }

    pub async fn set_workload(&mut self, workload: &WorkloadId, spec: WorkloadSpec) -> anyhow::Result<()> {
        self.store.write().unwrap().set_workload(workload, spec);
        // TODO: probably an indirection here
        self.scheduler.notify_changed(workload).await?;
        Ok(())
    }

    pub async fn remove_workload(&mut self, workload: &WorkloadId) -> anyhow::Result<()> {
        self.store.write().unwrap().remove_workload(workload);
        self.scheduler.notify_changed(workload).await?;
        Ok(())
    }
}
