use std::time::SystemTime;

use anyhow::anyhow;
use anyhow::Result;
use opentelemetry::global::ObjectSafeSpan;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::trace::Tracer;
use opentelemetry::Context;
use spin_core::async_trait;
use spin_core::wasmtime::component::Resource;
use spin_world::wasi::observe::tracer;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{GuestSpan, InstanceState};

#[async_trait]
impl tracer::Host for InstanceState {
    async fn start(
        &mut self,
        name: String,
        options: Option<tracer::StartOptions>,
    ) -> Result<Resource<tracer::Span>> {
        let mut state = self.state.write().unwrap();
        let options = options.unwrap_or_default();

        // Before we ever create any new spans make sure we track the original host span ID
        if state.original_host_span_id.is_none() {
            state.original_host_span_id = Some(
                tracing::Span::current()
                    .context()
                    .span()
                    .span_context()
                    .span_id(),
            );
        }

        // Get span's parent based on whether it's a new root and whether there are any active spans
        let parent_context = match (options.new_root, state.active_spans.is_empty()) {
            // Not a new root && Active spans -> Last active guest span is parent
            (false, false) => {
                let span_context = state
                    .guest_spans
                    .get(*state.active_spans.last().unwrap())
                    .unwrap()
                    .inner
                    .span_context()
                    .clone();
                Context::new().with_remote_span_context(span_context)
            }
            // Not a new root && No active spans -> Current host span is parent
            (false, true) => tracing::Span::current().context(),
            // New root && n/a -> No parent
            (true, _) => Context::new(),
        };

        // Create the underlying opentelemetry span
        let mut builder = self.tracer.span_builder(name);
        if let Some(kind) = options.span_kind {
            builder = builder.with_kind(kind.into());
        }
        if let Some(attributes) = options.attributes {
            builder = builder.with_attributes(attributes.into_iter().map(Into::into));
        }
        if let Some(links) = options.links {
            builder = builder.with_links(links.into_iter().map(Into::into).collect());
        }
        if let Some(timestamp) = options.timestamp {
            builder = builder.with_start_time(timestamp);
        }
        let otel_span = builder.start_with_context(&self.tracer, &parent_context);

        // Wrap it in a GuestSpan for our own bookkeeping purposes
        let guest_span = GuestSpan { inner: otel_span };

        // Put the GuestSpan in our resource table and push it on to our stack of active spans
        let resource_id = state.guest_spans.push(guest_span).unwrap();
        state.active_spans.insert(resource_id);

        Ok(Resource::new_own(resource_id))
    }
}

#[async_trait]
impl tracer::HostSpan for InstanceState {
    async fn span_context(
        &mut self,
        resource: Resource<tracer::Span>,
    ) -> Result<tracer::SpanContext> {
        if let Some(guest_span) = self.state.read().unwrap().guest_spans.get(resource.rep()) {
            Ok(guest_span.inner.span_context().clone().into())
        } else {
            Err(anyhow!("BUG: cannot find resource in table"))
        }
    }

    async fn is_recording(&mut self, resource: Resource<tracer::Span>) -> Result<bool> {
        if let Some(guest_span) = self.state.read().unwrap().guest_spans.get(resource.rep()) {
            Ok(guest_span.inner.is_recording())
        } else {
            Err(anyhow!("BUG: cannot find resource in table"))
        }
    }

    async fn set_attributes(
        &mut self,
        resource: Resource<tracer::Span>,
        attributes: Vec<tracer::KeyValue>,
    ) -> Result<()> {
        if let Some(guest_span) = self
            .state
            .write()
            .unwrap()
            .guest_spans
            .get_mut(resource.rep())
        {
            for attribute in attributes {
                guest_span.inner.set_attribute(attribute.into());
            }
            Ok(())
        } else {
            Err(anyhow!("BUG: cannot find resource in table"))
        }
    }

    async fn add_event(
        &mut self,
        resource: Resource<tracer::Span>,
        name: String,
        timestamp: Option<tracer::Datetime>,
        attributes: Option<Vec<tracer::KeyValue>>,
    ) -> Result<()> {
        if let Some(guest_span) = self
            .state
            .write()
            .unwrap()
            .guest_spans
            .get_mut(resource.rep())
        {
            let timestamp = timestamp.map(Into::into).unwrap_or_else(SystemTime::now);
            let attributes = if let Some(attributes) = attributes {
                attributes.into_iter().map(Into::into).collect()
            } else {
                vec![]
            };

            guest_span
                .inner
                .add_event_with_timestamp(name.into(), timestamp, attributes);

            Ok(())
        } else {
            Err(anyhow!("BUG: cannot find resource in table"))
        }
    }

    async fn add_link(
        &mut self,
        resource: Resource<tracer::Span>,
        link: tracer::Link,
    ) -> Result<()> {
        if let Some(guest_span) = self
            .state
            .write()
            .unwrap()
            .guest_spans
            .get_mut(resource.rep())
        {
            guest_span.inner.add_link(
                link.span_context.into(),
                link.attributes.into_iter().map(Into::into).collect(),
            );
            Ok(())
        } else {
            Err(anyhow!("BUG: cannot find resource in table"))
        }
    }

    async fn set_status(
        &mut self,
        resource: Resource<tracer::Span>,
        status: tracer::Status,
    ) -> Result<()> {
        if let Some(guest_span) = self
            .state
            .write()
            .unwrap()
            .guest_spans
            .get_mut(resource.rep())
        {
            guest_span.inner.set_status(status.into());
            Ok(())
        } else {
            Err(anyhow!("BUG: cannot find resource in table"))
        }
    }

    async fn update_name(&mut self, resource: Resource<tracer::Span>, name: String) -> Result<()> {
        if let Some(guest_span) = self
            .state
            .write()
            .unwrap()
            .guest_spans
            .get_mut(resource.rep())
        {
            guest_span.inner.update_name(name.into());
            Ok(())
        } else {
            Err(anyhow!("BUG: cannot find resource in table"))
        }
    }

    async fn end(
        &mut self,
        resource: Resource<tracer::Span>,
        timestamp: Option<tracer::Datetime>,
    ) -> Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(guest_span) = state.guest_spans.get_mut(resource.rep()) {
            if let Some(timestamp) = timestamp {
                guest_span.inner.end_with_timestamp(timestamp.into());
            } else {
                guest_span.inner.end();
            }

            // Remove the span from active_spans
            state.active_spans.shift_remove(&resource.rep());

            Ok(())
        } else {
            Err(anyhow!("BUG: cannot find resource in table"))
        }
    }

    async fn drop(&mut self, resource: Resource<tracer::Span>) -> Result<()> {
        // Dropping the resource automatically calls drop on the Span which ends itself with the
        // current timestamp if the Span is not already ended.

        // Ensure that the span has been removed from active_spans
        let mut state = self.state.write().unwrap();
        state.active_spans.shift_remove(&resource.rep());

        Ok(())
    }
}
