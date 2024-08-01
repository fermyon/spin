use std::path::PathBuf;

use spin_factor_wasi::{spin::SpinFilesMounter, WasiFactor};
use spin_factors::RuntimeFactors;

#[derive(RuntimeFactors)]
pub struct TriggerFactors {
    pub wasi: WasiFactor,
}

impl TriggerFactors {
    pub fn new(working_dir: impl Into<PathBuf>, allow_transient_writes: bool) -> Self {
        let files_mounter = SpinFilesMounter::new(working_dir, allow_transient_writes);
        Self {
            wasi: WasiFactor::new(files_mounter),
        }
    }
}
