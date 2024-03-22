use std::sync::{Arc, RwLock};

use anyhow::Result;
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::wasmtime::component::Resource;
use spin_core::{async_trait, HostComponent};
use spin_world::v2::observe;
use spin_world::v2::observe::Span as WitSpan;

pub struct ObserveHostComponent {}

impl ObserveHostComponent {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {}
    }
}

impl HostComponent for ObserveHostComponent {
    type Data = ObserveData;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        observe::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        ObserveData {
            state: Arc::new(RwLock::new(State {
                guest_spans: table::Table::new(1024),
                active_spans: Default::default(),
            })),
        }
    }
}

impl DynamicHostComponent for ObserveHostComponent {
    fn update_data(&self, _data: &mut Self::Data, _component: &AppComponent) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct ObserveData {
    pub(crate) state: Arc<RwLock<State>>,
}

#[async_trait]
impl observe::Host for ObserveData {}

#[async_trait]
impl observe::HostSpan for ObserveData {
    async fn enter(&mut self, name: String) -> Result<Resource<WitSpan>> {
        // Create the underlying tracing span
        let tracing_span = tracing::info_span!("WASI Observe guest", "otel.name" = name);

        // Wrap it in a GuestSpan for our own bookkeeping purposes and enter it
        let guest_span = GuestSpan {
            name: name.clone(),
            inner: tracing_span,
        };
        guest_span.enter();

        // Put the GuestSpan in our resource table and push it to our stack of active spans
        let mut state = self.state.write().unwrap();
        let resource_id = state.guest_spans.push(guest_span).unwrap();
        state.active_spans.push(resource_id);

        Ok(Resource::new_own(resource_id))
    }

    async fn close(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        self.safely_close(resource, false);
        Ok(())
    }

    fn drop(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        self.safely_close(resource, true);
        Ok(())
    }
}

impl ObserveData {
    /// Close the span associated with the given resource and optionally drop the resource
    /// from the table. Additionally close any other active spans that are more recent on the stack
    /// in reverse order.
    ///
    /// Exiting any spans that were already closed will not cause this to error.
    fn safely_close(&mut self, resource: Resource<WitSpan>, drop_resource: bool) {
        let mut state: std::sync::RwLockWriteGuard<State> = self.state.write().unwrap();

        if let Some(index) = state
            .active_spans
            .iter()
            .rposition(|id| *id == resource.rep())
        {
            state.close_from_back_to(index);
        } else {
            tracing::debug!("found no active spans to close")
        }

        if drop_resource {
            state.guest_spans.remove(resource.rep()).unwrap();
        }
    }
}

/// Internal state of the observe host component.
pub(crate) struct State {
    /// A resource table that holds the guest spans.
    pub guest_spans: table::Table<GuestSpan>,
    /// A LIFO stack of guest spans that are currently active.
    ///
    /// Only a reference ID to the guest span is held here. The actual guest span must be looked up
    /// in the `guest_spans` table using the reference ID.
    pub active_spans: Vec<u32>,
}

impl State {
    /// Close all active spans from the top of the stack to the given index. Closing entails exiting
    /// the inner [tracing] span and removing it from the active spans stack.
    pub(crate) fn close_from_back_to(&mut self, index: usize) {
        self.active_spans
            .split_off(index)
            .iter()
            .rev()
            .for_each(|id| {
                if let Some(guest_span) = self.guest_spans.get(*id) {
                    guest_span.exit();
                } else {
                    tracing::debug!("active_span {id:?} already removed from resource table");
                }
            });
    }

    /// Enter the inner [tracing] span for all active spans.
    pub(crate) fn enter_all(&self) {
        for guest_span_id in self.active_spans.iter() {
            if let Some(span_resource) = self.guest_spans.get(*guest_span_id) {
                span_resource.enter();
            } else {
                tracing::debug!("guest span already dropped")
            }
        }
    }

    /// Exit the inner [tracing] span for all active spans.
    pub(crate) fn exit_all(&self) {
        for guest_span_id in self.active_spans.iter().rev() {
            if let Some(span_resource) = self.guest_spans.get(*guest_span_id) {
                span_resource.exit();
            } else {
                tracing::debug!("guest span already dropped")
            }
        }
    }
}

/// The WIT resource Span. Effectively wraps a [tracing] span.
pub struct GuestSpan {
    /// The [tracing] span we use to do the actual tracing work.
    pub inner: tracing::Span,
    pub name: String,
}

// Note: We use tracing enter instead of Entered because Entered is not Send
impl GuestSpan {
    /// Enter the inner [tracing] span.
    pub fn enter(&self) {
        self.inner.with_subscriber(|(id, dispatch)| {
            dispatch.enter(id);
        });
    }

    /// Exits the inner [tracing] span.
    pub fn exit(&self) {
        self.inner.with_subscriber(|(id, dispatch)| {
            dispatch.exit(id);
        });
    }
}
