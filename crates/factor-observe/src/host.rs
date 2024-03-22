use anyhow::Result;
use opentelemetry::trace::TraceContextExt;
use spin_core::async_trait;
use spin_core::wasmtime::component::Resource;
use spin_world::wasi::clocks0_2_0::wall_clock::Datetime;
use spin_world::wasi::observe::traces::{self, KeyValue, Span as WitSpan};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{GuestSpan, InstanceState};

#[async_trait]
impl traces::Host for InstanceState {}

#[async_trait]
impl traces::HostSpan for InstanceState {
    async fn start(&mut self, name: String) -> Result<Resource<WitSpan>> {
        // Create the underlying tracing span
        let tracing_span = tracing::info_span!("WASI Observe guest", "otel.name" = name);
        let span_id = tracing_span
            .context()
            .span()
            .span_context()
            .span_id()
            .to_string();

        // Wrap it in a GuestSpan for our own bookkeeping purposes and enter it
        let guest_span = GuestSpan {
            name: name.clone(),
            inner: tracing_span,
        };
        guest_span.enter();

        // Put the GuestSpan in our resource table and push it to our stack of active spans
        let mut state = self.state.write().unwrap();
        let resource_id = state.guest_spans.push(guest_span).unwrap();
        state.active_spans.insert(span_id, resource_id);

        Ok(Resource::new_own(resource_id))
    }

    async fn set_attribute(
        &mut self,
        resource: Resource<WitSpan>,
        attribute: KeyValue,
    ) -> Result<()> {
        if let Some(guest_span) = self
            .state
            .try_write()
            .unwrap()
            .guest_spans
            .get_mut(resource.rep())
        {
            guest_span
                .inner
                .set_attribute(attribute.key, attribute.value);
        } else {
            tracing::debug!("can't find guest span to set attribute on")
        }
        Ok(())
    }

    async fn set_attributes(
        &mut self,
        resource: Resource<WitSpan>,
        attributes: Vec<KeyValue>,
    ) -> Result<()> {
        if let Some(guest_span) = self
            .state
            .write()
            .unwrap()
            .guest_spans
            .get_mut(resource.rep())
        {
            for attribute in attributes {
                guest_span
                    .inner
                    .set_attribute(attribute.key, attribute.value);
            }
        } else {
            tracing::debug!("can't find guest span to set attributes on")
        }
        Ok(())
    }

    async fn add_event(
        &mut self,
        _resource: Resource<WitSpan>,
        _name: String,
        _timestamp: Option<Datetime>,
        _attributes: Option<Vec<KeyValue>>,
    ) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    async fn end(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        self.safely_close(resource.rep(), false);
        Ok(())
    }

    fn drop(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        self.safely_close(resource.rep(), true);
        Ok(())
    }
}
