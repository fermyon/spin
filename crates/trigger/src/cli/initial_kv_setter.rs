use anyhow::Context as _;
use spin_core::async_trait;
use spin_factor_key_value::KeyValueFactor;
use spin_factors::RuntimeFactors;
use spin_factors_executor::ExecutorHooks;

/// An [`ExecutorHooks`] that sets initial key-value pairs in the default store.
pub struct InitialKvSetterHook {
    kv_pairs: Vec<(String, String)>,
}

impl InitialKvSetterHook {
    pub fn new(kv_pairs: Vec<(String, String)>) -> Self {
        Self { kv_pairs }
    }
}

const DEFAULT_KEY_VALUE_STORE_LABEL: &str = "default";

#[async_trait]
impl<F: RuntimeFactors, U> ExecutorHooks<F, U> for InitialKvSetterHook {
    async fn configure_app(
        &self,
        configured_app: &spin_factors::ConfiguredApp<F>,
    ) -> anyhow::Result<()> {
        if self.kv_pairs.is_empty() {
            return Ok(());
        }
        let kv = configured_app.app_state::<KeyValueFactor>().context(
            "attempted to set initial kv pairs but the key-value factor was not configured",
        )?;
        let store = kv
            .get_store(DEFAULT_KEY_VALUE_STORE_LABEL)
            .await
            .expect("trigger was misconfigured and lacks a default store");
        for (key, value) in &self.kv_pairs {
            store
                .set(key, value.as_bytes())
                .await
                .context("failed to set key-value pair")?;
        }

        Ok(())
    }
}
