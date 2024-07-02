title = "SIP 0XX - Spin Factors"
template = "main"
date = "2024-05-20T12:00:00Z"

---

Summary: Refactor Spin Runtime Functionality with "Factors"

Owner: lann.martin@fermyon.com

Created: May 20, 2024

## Background

Spin 1.0 shipped with a mechanism for extending the runtime environment with
loosely-coupled host functionality called
[`HostComponent`](https://fermyon.github.io/rust-docs/spin/v2.2.0/spin_core/trait.HostComponent.html).
As Spin has evolved, more and more runtime functionality has fallen outside of
the scope of this mechanism, leading to the code for these features spreading
out over the Spin codebase. This not only makes it hard to read and understand
these features but also raises the bar for unfamiliar developers trying to add
new features.

Separately, the introduction of the SpinKube project has made it clear that
there will be (at least) two major embeddings of the Spin runtime environment:
the Spin CLI (`spin up`) and SpinKube (`containerd-shim-spin`). In order to
provide better integration into the Kubernetes ecosystem, some runtime feature
implementations will diverge between these embeddings, making the loose coupling
of features even more important.

## Proposal

The basic inversion of control approach used by `HostComponent`s will be
redesigned and expanded into a new system called Spin Factors, where independent
feature sets are organized into individual "factors". A "factor" encapsulates a
single logical Spin runtime feature into a reusable Rust crate, using the Spin
Factors framework to "hook" into various points in the lifecycle of a Spin
trigger's execution.

Some of the goals of this new system:

- Expand the set integration points that a factor can use to hook into the
  lifecycle of an application request. These will be driven by feature
  requirements and may include e.g.:
  - Runtime startup
  - Application initialization
  - Runtime config parsing
  - Component pre-instantiation
  - Component instantiation
  - Cleanup (post-execution)

- Allow loosely-coupled dependencies between "factors" (features). In order to
  simplify implementation and reasoning about these dependencies the dependency
  graph must be acyclic.

## Implementation Plan

The overall implementation plan will be to (in parallel):

- Introduce a new `spin-factors` crate containing the basic framework.
- Refactor existing Spin features (mostly existing `HostComponent`s) to use
   this new framework.
   - Framework features will be developed as needed to support this refactoring.
- Refactor `spin-core`, `spin-trigger`, etc. to use `spin-factors`.
   - Depending on subjective evaluation at this point, possibly merge
   `spin-core` and `spin-factors`.

## Implementation Details

Based on initial prototyping, the following Rust types represent the starting
point for `spin-factors` (simplified from the actual code for clarity):

```rust
pub trait Factor {
    // This provides a mechanism to configure this factor on a per-app basis
    // based on "runtime config", as currently implemented by
    // `spin_trigger::RuntimeConfig`.
    type RuntimeConfig;

    // This stores per-app state for the factor; see `configure_app` below.
    // This state *may* be cached by the runtime across multiple requests.
    type AppState;

    // This type is used to build per-instance (i.e. per-request) state; see
    // `FactorInstanceBuilder` below.
    type InstanceBuilder: FactorInstanceBuilder;

    // Initializes the factor once at runtime startup. `InitContext` provides
    // access to the wasmtime `Linker`, so this is where any bindgen
    // `add_to_linker` calls go.
    fn init(&mut self, ctx: InitContext<Factors, Self>) -> Result<()> {
        Ok(())
    }

    // This validates and uses app (manifest) and runtime configuration to build
    // `Self::AppState` to be used in `prepare` below.
    // 
    // `ConfigureAppContext` gives access to:
    // - The `spin_app::App`
    // - This factors's `RuntimeConfig`
    // - The `AppState` for any factors configured before this one
    //
    // These methods can also be used on their own (without `init` or `prepare`)
    // to just validate app configuration for e.g. `spin doctor`.
    fn configure_app(&self, ctx: ConfigureAppContext) -> Result<Self::AppState>;

    // Creates a new `FactorInstanceBuilder`, which will later build per-instance
    // state for this factor.
    //
    // `PrepareContext` gives access to the `spin_app::AppComponent` and this
    // factor's `AppState`.
    //
    // This is primary place for inter-factor dependencies to be used via the
    // provided `InstanceBuilders` which gives access to the `InstanceBuilder`s
    // of any factors prepared before this one.
    fn prepare(ctx: PrepareContext, builders: &mut InstanceBuilders) -> Result<Self::InstanceBuilder>;
}

pub trait FactorInstanceBuilder {
    // This instance state is built per-component-instance (per-request).
    //
    // This is equivalent to the existing `HostComponent::Data` and ends up
    // being stored in the `wasmtime::Store`. Any `bindgen` traits for this
    // factor will be implemented on this type.
    type InstanceState;

    fn build(self) -> Result<Self::InstanceState>;
}
```
