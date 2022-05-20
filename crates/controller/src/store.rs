use std::collections::HashMap;

use crate::schema::{WorkloadId, WorkloadSpec};

pub(crate) trait WorkStore {
    fn set_workload(&mut self, workload: &WorkloadId, spec: WorkloadSpec);
    fn remove_workload(&mut self, workload: &WorkloadId);
    fn get_workload(&self, workload: &WorkloadId) -> Option<WorkloadSpec>;
}

pub(crate) struct InMemoryWorkStore {
    workloads: HashMap<WorkloadId, WorkloadSpec>,
}

impl InMemoryWorkStore {
    pub(crate) fn new() -> Self {
        Self {
            workloads: HashMap::new()
        }
    }
}

impl WorkStore for InMemoryWorkStore {
    fn set_workload(&mut self, workload: &WorkloadId, spec: WorkloadSpec) {
        self.workloads.insert(workload.clone(), spec);
    }

    fn remove_workload(&mut self, workload: &WorkloadId) {
        self.workloads.remove(workload);
    }

    fn get_workload(&self, workload: &WorkloadId) -> Option<WorkloadSpec> {
        self.workloads.get(workload).map(|w| w.clone())
    }
}
