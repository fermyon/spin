use std::{ascii::escape_default, sync::OnceLock};

use crate::env;

/// Takes a Spin application log and emits it as a tracing event.
///
/// This acts as a compatibility layer to easily get Spin app logs as events in our OTel traces.
pub fn app_log_to_tracing_event(buf: &[u8]) {
    static CELL: OnceLock<bool> = OnceLock::new();
    if *CELL.get_or_init(env::spin_disable_log_to_tracing) {
        return;
    }

    if let Ok(s) = std::str::from_utf8(buf) {
        tracing::info!(app_log = s);
    } else {
        tracing::info!(
            app_log_non_utf8 = buf
                .iter()
                .take(50)
                .map(|&x| escape_default(x).to_string())
                .collect::<String>()
        );
    }
}
