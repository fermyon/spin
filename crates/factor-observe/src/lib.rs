pub mod future;
mod host;

use std::sync::{Arc, RwLock};

use future::ObserveContext;
use indexmap::IndexMap;
use spin_factors::{Factor, SelfInstanceBuilder};

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
        ctx.link_bindings(spin_world::wasi::observe::traces::add_to_linker)?;
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
        _ctx: spin_factors::PrepareContext<Self>,
        _builders: &mut spin_factors::InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        Ok(InstanceState {
            state: Arc::new(RwLock::new(State {
                guest_spans: table::Table::new(1024),
                active_spans: Default::default(),
            })),
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
}

impl SelfInstanceBuilder for InstanceState {}

impl InstanceState {
    /// Close the span associated with the given resource and optionally drop the resource
    /// from the table. Additionally close any other active spans that are more recent on the stack
    /// in reverse order.
    ///
    /// Exiting any spans that were already closed will not cause this to error.
    fn safely_close(&mut self, resource_id: u32, drop_resource: bool) {
        let mut state: std::sync::RwLockWriteGuard<State> = self.state.write().unwrap();

        if let Some(index) = state
            .active_spans
            .iter()
            .rposition(|(_, id)| *id == resource_id)
        {
            state.close_from_back_to(index);
        } else {
            tracing::debug!("found no active spans to close")
        }

        if drop_resource {
            state.guest_spans.remove(resource_id).unwrap();
        }
    }

    pub fn get_observe_context(&self) -> ObserveContext {
        ObserveContext {
            state: self.state.clone(),
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
    /// TODO: Fix comment
    pub active_spans: IndexMap<String, u32>,
}

impl State {
    /// Close all active spans from the top of the stack to the given index. Closing entails exiting
    /// the inner [tracing] span and removing it from the active spans stack.
    pub(crate) fn close_from_back_to(&mut self, index: usize) {
        self.active_spans
            .split_off(index)
            .iter()
            .rev()
            .for_each(|(_, id)| {
                if let Some(guest_span) = self.guest_spans.get(*id) {
                    guest_span.exit();
                } else {
                    tracing::debug!("active_span {id:?} already removed from resource table");
                }
            });
    }

    /// Enter the inner [tracing] span for all active spans.
    pub(crate) fn enter_all(&self) {
        for (_, guest_span_id) in self.active_spans.iter() {
            if let Some(span_resource) = self.guest_spans.get(*guest_span_id) {
                span_resource.enter();
            } else {
                tracing::debug!("guest span already dropped")
            }
        }
    }

    /// Exit the inner [tracing] span for all active spans.
    pub(crate) fn exit_all(&self) {
        for (_, guest_span_id) in self.active_spans.iter().rev() {
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
