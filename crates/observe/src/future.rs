use anyhow::{Context, Result};
use pin_project_lite::pin_project;
use std::{
    future::Future,
    sync::{Arc, RwLock},
};

use spin_core::{Engine, Store};

use crate::{host_component::State, ObserveHostComponent};

pin_project! {
    struct Instrumented<F> {
        #[pin]
        inner: F,
        observe_context: ObserveContext,
    }

    impl<F> PinnedDrop for Instrumented<F> {
        fn drop(this: Pin<&mut Self>) {
            this.project().observe_context.drop_all();
        }
    }
}

pub trait FutureExt: Future + Sized {
    /// Manage WASI Observe guest spans.
    fn manage_guest_spans(
        self,
        observe_context: ObserveContext,
    ) -> Result<impl Future<Output = Self::Output>>;
}

impl<F: Future> FutureExt for F {
    fn manage_guest_spans(
        self,
        observe_context: ObserveContext,
    ) -> Result<impl Future<Output = Self::Output>> {
        Ok(Instrumented {
            inner: self,
            observe_context,
        })
    }
}

impl<F: Future> Future for Instrumented<F> {
    type Output = F::Output;

    /// Maintains the invariant that all active spans are entered before polling the inner future
    /// and exited otherwise. If we don't do this then the timing (among many other things) of the
    /// spans becomes wildly incorrect.
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();

        // Enter the active spans before entering the inner poll
        {
            this.observe_context.state.write().unwrap().enter_all();
        }

        let ret = this.inner.poll(cx);

        // Exit the active spans after exiting the inner poll
        {
            this.observe_context.state.write().unwrap().exit_all();
        }

        ret
    }
}

/// The context necessary for the observe host component to function.
pub struct ObserveContext {
    state: Arc<RwLock<State>>,
}

impl ObserveContext {
    pub fn new<T>(store: &mut Store<T>, engine: &Engine<T>) -> Result<Self> {
        let handle = engine
            .find_host_component_handle::<Arc<ObserveHostComponent>>()
            .context("host component handle not found")?;
        let state = store
            .host_components_data()
            .get_or_insert(handle)
            .state
            .clone();
        Ok(Self { state })
    }

    fn drop_all(&self) {
        self.state.write().unwrap().close_from_back_to(0);
    }
}
