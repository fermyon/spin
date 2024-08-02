use std::path::PathBuf;

use spin_factor_outbound_http::OutboundHttpFactor;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_wasi::{spin::SpinFilesMounter, WasiFactor};
use spin_factors::RuntimeFactors;

#[derive(RuntimeFactors)]
pub struct TriggerFactors {
    pub wasi: WasiFactor,
    pub outbound_networking: OutboundNetworkingFactor,
    pub outbound_http: OutboundHttpFactor,
}

impl TriggerFactors {
    pub fn new(working_dir: impl Into<PathBuf>, allow_transient_writes: bool) -> Self {
        let files_mounter = SpinFilesMounter::new(working_dir, allow_transient_writes);
        Self {
            wasi: WasiFactor::new(files_mounter),
            outbound_networking: OutboundNetworkingFactor,
            outbound_http: OutboundHttpFactor,
        }
    }
}
