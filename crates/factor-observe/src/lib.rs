mod host;

use std::sync::{Arc, RwLock};

use indexmap::IndexSet;
use opentelemetry::{
    global::{self, BoxedTracer, ObjectSafeSpan},
    trace::{SpanId, TraceContextExt},
    Context,
};
use spin_factors::{Factor, PrepareContext, RuntimeFactors, SelfInstanceBuilder};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Default)]
pub struct ObserveFactor {}

impl Factor for ObserveFactor {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::wasi::observe::tracer::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: spin_factors::RuntimeFactors>(
        &self,
        _ctx: spin_factors::ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        Ok(())
    }

    fn prepare<T: spin_factors::RuntimeFactors>(
        &self,
        ctx: spin_factors::PrepareContext<T, Self>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let tracer = global::tracer(ctx.app_component().app.id().to_string());
        Ok(InstanceState {
            state: Arc::new(RwLock::new(State {
                guest_spans: Default::default(),
                active_spans: Default::default(),
                original_host_span_id: None,
            })),
            tracer,
        })
    }
}

impl ObserveFactor {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct InstanceState {
    pub(crate) state: Arc<RwLock<State>>,
    pub(crate) tracer: BoxedTracer,
}

impl SelfInstanceBuilder for InstanceState {}

/// Internal state of the ObserveFactor instance state.
///
/// This data lives here rather than directly on InstanceState so that we can have multiple things
/// take Arc references to it.
pub(crate) struct State {
    /// A resource table that holds the guest spans.
    pub(crate) guest_spans: spin_resource_table::Table<GuestSpan>,

    /// A stack of resource ids for all the active guest spans. The topmost span is the active span.
    ///
    /// When a guest span is ended it is removed from this stack (regardless of whether is the
    /// active span) and all other spans are shifted back to retain relative order.
    pub(crate) active_spans: IndexSet<u32>,

    /// Id of the last span emitted from within the host before entering the guest.
    ///
    /// We use this to avoid accidentally reparenting the original host span as a child of a guest
    /// span.
    pub(crate) original_host_span_id: Option<SpanId>,
}

/// The WIT resource Span. Effectively wraps an [opentelemetry::global::BoxedSpan].
pub struct GuestSpan {
    /// The [opentelemetry::global::BoxedSpan] we use to do the actual tracing work.
    pub inner: opentelemetry::global::BoxedSpan,
}

/// Manages access to the ObserveFactor state for the purpose of maintaining proper span
/// parent/child relationships when WASI Observe spans are being created.
pub struct ObserveContext {
    pub(crate) state: Option<Arc<RwLock<State>>>,
}

impl ObserveContext {
    /// Creates an [`ObserveContext`] from a [`PrepareContext`].
    ///
    /// If [`RuntimeFactors`] does not contain an [`ObserveFactor`], then calling
    /// [`ObserveContext::reparent_tracing_span`] will be a no-op.
    pub fn from_prepare_context<T: RuntimeFactors, F: Factor>(
        prepare_context: &mut PrepareContext<T, F>,
    ) -> anyhow::Result<Self> {
        let state = match prepare_context.instance_builder::<ObserveFactor>() {
            Ok(instance_state) => Some(instance_state.state.clone()),
            Err(spin_factors::Error::NoSuchFactor(_)) => None,
            Err(e) => return Err(e.into()),
        };
        Ok(Self { state })
    }

    /// Reparents the current [tracing] span to be a child of the last active guest span.
    ///
    /// The observe factor enables guests to emit spans that should be part of the same trace as the
    /// host is producing for a request. Below is an example trace. A request is made to an app, a
    /// guest span is created and then the host is re-entered to fetch a key value.
    ///
    /// ```text
    /// | GET /... _________________________________|
    ///    | execute_wasm_component foo ___________|
    ///       | my_guest_span ___________________|
    ///          | spin_key_value.get |
    /// ```
    ///
    ///  Setting the guest spans parent as the host is trivially done. However, the more difficult
    /// task is having the host factor spans be children of the guest span.
    /// [`ObserveContext::reparent_tracing_span`] handles this by reparenting the current span to be
    /// a child of the last active guest span (which is tracked internally in the observe factor).
    ///
    /// Note that if the observe factor is not in your [`RuntimeFactors`] than this is effectively a
    /// no-op.
    ///
    /// This MUST only be called from a factor host implementation function that is instrumented.
    ///
    /// This MUST be called at the very start of the function before any awaits.
    pub fn reparent_tracing_span(&self) {
        // If state is None then we want to return early b/c the factor doesn't depend on the
        // Observe factor and therefore there is nothing to do
        let state = if let Some(state) = self.state.as_ref() {
            state.read().unwrap()
        } else {
            return;
        };

        // If there are no active guest spans then there is nothing to do
        let Some(current_span_id) = state.active_spans.last() else {
            return;
        };

        // Ensure that we are not reparenting the original host span
        if let Some(original_host_span_id) = state.original_host_span_id {
            if tracing::Span::current()
                .context()
                .span()
                .span_context()
                .span_id()
                .eq(&original_host_span_id)
            {
                panic!("Incorrectly attempting to reparent the original host span. Likely `reparent_tracing_span` was called in an incorrect location.")
            }
        }

        // Now reparent the current span to the last active guest span
        let span_context = state
            .guest_spans
            .get(*current_span_id)
            .unwrap()
            .inner
            .span_context()
            .clone();
        let parent_context = Context::new().with_remote_span_context(span_context);
        tracing::Span::current().set_parent(parent_context);
    }
}
