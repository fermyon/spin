use spin_app::AppComponent;
use spin_core::StoreBuilder;

use crate::TriggerHooks;

pub struct EventId;

impl TriggerHooks for EventId {
    fn component_store_builder(
        &self,
        _component: &AppComponent,
        store_builder: &mut StoreBuilder,
    ) -> anyhow::Result<()> {
        let event_id = uuid::Uuid::new_v4().to_string();
        store_builder.env([("SPIN_EVENT_ID", &event_id)])?;
        Ok(())
    }
}
