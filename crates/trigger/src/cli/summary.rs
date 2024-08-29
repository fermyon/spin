use spin_factor_key_value::KeyValueFactor;
use spin_factors_executor::ExecutorHooks;

use crate::factors::TriggerFactors;

pub struct KeyValueDefaultStoreSummaryHook;

impl<U> ExecutorHooks<TriggerFactors, U> for KeyValueDefaultStoreSummaryHook {
    fn configure_app(
        &mut self,
        configured_app: &spin_factors::ConfiguredApp<TriggerFactors>,
    ) -> anyhow::Result<()> {
        if let Some(default_store_summary) = configured_app
            .app_state::<KeyValueFactor>()
            .ok()
            .and_then(|kv_state| kv_state.store_summary("default"))
        {
            println!("Storing default key-value data to {default_store_summary}.");
        }
        Ok(())
    }
}
