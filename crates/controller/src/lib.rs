use std::sync::{RwLock, Arc};

use scheduler::{Scheduler, LocalScheduler};
pub use schema::{WorkloadId, WorkloadSpec};
use store::{WorkStore, InMemoryWorkStore};

pub(crate) mod scheduler;
pub(crate) mod schema;
pub(crate) mod store;

pub struct Control {
    scheduler: Box<dyn Scheduler>,
    store: Arc<RwLock<Box<dyn WorkStore>>>,
}

impl Control {
    pub fn in_memory() -> Self {
        let box_store: Box<dyn WorkStore> = Box::new(InMemoryWorkStore::new());
        let store = Arc::new(RwLock::new(box_store));
        Self {
            scheduler: Box::new(LocalScheduler::new(store.clone())),
            store,
        }
    }

    pub fn set_workload(&mut self, workload: &WorkloadId, spec: WorkloadSpec) {
        self.store.write().unwrap().set_workload(workload, spec);
        // TODO: probably an indirection here
        self.scheduler.notify_changed(workload);
    }

    pub fn remove_workload(&mut self, _workload: &WorkloadId) {
        todo!()
    }
}
