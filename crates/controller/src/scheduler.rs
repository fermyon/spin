use std::{collections::HashMap, sync::{Arc, RwLock}};

use spin_http_engine::HttpTrigger;
use spin_redis_engine::RedisTrigger;
use spin_trigger::Trigger;

use crate::{schema::WorkloadId, store::WorkStore, WorkloadSpec};

pub(crate) trait Scheduler {
    fn notify_changed(&self, workload: &WorkloadId);
}

pub(crate) struct LocalScheduler {
    store: Arc<RwLock<Box<dyn WorkStore>>>,
    running: Arc<RwLock<HashMap<WorkloadId, RunningWorkload>>>,
}

impl LocalScheduler {
    pub(crate) fn new(store: Arc<RwLock<Box<dyn WorkStore>>>) -> Self {
        Self {
            store,
            running: Arc::new(RwLock::new(HashMap::new()))
        }
    }
}

enum RunningWorkload {
    // InProcessTrigger(Box<dyn Trigger>),
    // InProcessHttp(HttpTrigger),
    // InProcessRedis(RedisTrigger),
    Fut(core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<()>>>>),
}

impl Scheduler for LocalScheduler {
    fn notify_changed(&self, workload: &WorkloadId) {
        let spec = self.store.read().unwrap().get_workload(workload);
        let running = self.running.read().unwrap();
        let current = running.remove(workload);

        match (spec, current) {
            (Some(w), Some(c)) => self.restart_workload(workload, w, c),
            (Some(w), None) => self.start_workload(workload, w),
            (None, Some(c)) => self.stop_workload(workload, c),
            (None, None) => (),
        }
    }
}

impl LocalScheduler {
    fn start_workload(&self, workload: &WorkloadId, spec: WorkloadSpec) {
        // Identify the application type
        // Instantiate the relevant trigger
        // Start the relevant trigger
        let fut = trigger.run();
        // Stash the task
        let running = RunningWorkload::Fut(fut);
        self.running.write().unwrap().insert(workload.clone(), running);
    }

    fn restart_workload(&self, workload: &WorkloadId, spec: WorkloadSpec, current: RunningWorkload) {
        self.stop_workload(workload, current);
        self.start_workload(workload, spec);
    }

    fn stop_workload(&self, workload: &WorkloadId, current: RunningWorkload) {
        current.stop();
        self.running.write().unwrap().remove(workload);
    }
}

impl RunningWorkload {
    fn stop(self) {
        match self {
            Self::Fut(f) => drop(f),
        }
    }
}
