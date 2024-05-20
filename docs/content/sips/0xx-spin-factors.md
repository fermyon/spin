title = "SIP 0XX - Spin Factors"
template = "main"
date = "2024-05-20T12:00:00Z"

---

Summary: Refactor Spin Runtime Functionality with "Factors"

Owner: lann.martin@fermyon.com

Created: May 20, 2024

## Background

Spin 1.0 shipped with a mechanism for extending the runtime environment with
loosely-coupled host functionality called "host components". As Spin has
evolved, more and more runtime functionality has fallen outside of the scope of
this mechanism, leading to the code for these features spreading out over the
Spin codebase. This not only makes it hard to read and understand these features
but also raises the bar for unfamiliar developers trying to add new features.

Separately, the introduction of the SpinKube project has made it clear that
there will be (at least) two major embeddings of the Spin runtime environment:
the Spin CLI (`spin up`) and SpinKube (`containerd-shim-spin`). In order to
provide better integration into the Kubernetes ecosystem, some runtime feature
implementations will diverge between these embeddings, making the loose coupling
of features even more important.

## Proposal

The basic inversion of control approach used by `HostComponent`s will be
redesigned and expanded into a new system called Spin Factors, where independent
feature sets are organized into individual "factors". Some of the goals of this
new system:

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

The overall implementation plan will be to:

1. Introduce a new `spin-factors` crate containing the basic framework.

2. Refactor existing Spin features (mostly existing `HostComponent`s) to use
   this new framework.
   - Framework features will be developed as needed to support this refactoring.

3. Refactor `spin-core`, `spin-trigger`, etc. to use `spin-factors`.
   - Depending on subjective evaluation at this point, possibly merge
   `spin-core` and `spin-factors`.

## Implementation Details

Based on initial prototyping, the following Rust types represent the starting
point for `spin-factors` (note that some type details are elided for clarity):

```rust
pub trait Factor {
    // Builder represents a type that can prepare request state for the factor.
    type Builder: FactorBuilder<Self>;
    // Data represents request state for the factor.
    type Data;

    // Init is the runtime startup lifecycle hook.
    //
    // The `InitContext` type here gives the factor the ability to update global
    // engine configuration, most notably the `Linker`. This takes the place of
    // `HostComponent::add_to_linker`.
    fn init(ctx: InitContext) -> Result<()> {
        Ok(())
    }

    // Validate app is the application initialization hook.
    //
    // This takes the place of `DynamicHostComponent::validate_app`.
    fn validate_app(&self, app: &App) -> Result<()> {
        Ok(())
    }
}

pub trait FactorBuilder<Factor> {
    // Prepare is the component pre-instantiation hook.
    //
    // The `PrepareContext` type gives access to information about the Spin app
    // and component being prepared and also to the Factor itself and any other
    // already-`prepare`d `FactorBuilder`s. The return value is the request state
    // builder for this factor. This builder may expose mutable state to other
    // factors, providing inter-factor dependency features. This takes the place
    // of `DynamicHostComponent::update_data`.
    fn prepare(ctx: PrepareContext) -> Result<Self>;

    // Build is the component instantiation hook.
    //
    // This returns the request state for this factor. This takes the place of
    // `HostComponent::build_data`.
    fn build(self) -> Factor::Data;
}
```