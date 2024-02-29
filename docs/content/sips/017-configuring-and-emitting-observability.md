title = "SIP 017 - Configuring and Emitting Observability"
template = "main"
date = "2024-02-27T12:00:00Z"

---

Summary: How to configure and emit telemetry to improve the runtime and trigger observability of Spin.

Owner: caleb.schoepp@fermyon.com

Created: February 27, 2024

Updated: February 27, 2024

## Background

[Observability](https://opentelemetry.io/docs/concepts/observability-primer/#what-is-observability) is critical for a great developer experience. Improving the observability of Spin can be broken down into four categories:

1. Runtime observability: Observing the Spin runtime itself e.g. spans and metrics around important parts of the Spin runtime.
2. Trigger observability: Observing the requests made to Spin applications e.g. spans and metrics around the requests made to Spin applications.
3. Component observability: Observing the interaction between composed components e.g having Wasmtime auto-instrument Wasm component graphs to produce spans.
4. Guest observability: Observing code within the guest module e.g. building a host component for WASI Observe.

More detail on these categories can be found [here](https://github.com/fermyon/spin/issues/2293).

## Proposal

This SIP aims to improve the runtime and trigger observability of Spin. First, it will suggest how observability in Spin can be configured. Second, it will present the specific runtime and trigger observability we want to emit. Finally, it will outline a plan going forward for how observability should be added to new Spin features.

This proposal assumes that all observability data produced should be OTEL compliant.

### Configuring observability

The developer must be able to configure the endpoints of OTLP compliant collectors where traces and metrics can be sent. This can be expressed in the `runtime-config.toml`.

```toml
[observability.tracing]
endpoint = "http://localhost:4317"
protocol = "http/protobuf" # "grpc" or "http/protobuf" or "http/json" and it defaults to "http/protobuf"
[observability.metrics]
endpoint = "http://localhost:4317"
protocol = "http/protobuf" # "grpc" or "http/protobuf" or "http/json" and it defaults to "http/protobuf"
```

**BIKESHED:** `observability.tracing`, `otel.tracing`, `telemetry.tracing`....

The developer must be able to configure how each component of their application handles tracing. This can be expressed in the `spin.toml` manifest.

```toml
[component.my-component.tracing]
context_propagation = true # This is all or nothing. If you disable propagation no context will be propagated. By default this is false.
# Opportunity to add fields in the future to
# - Disable tracing for performance reasons
# - Customize span names
# - Add additional metadata
# - More complex allow-listing mechanism for what spans propagate
# - etc.
```

### Improving observability

#### Runtime observability

Currently a few spans are emitted by Spin, but there is no consistent pattern. We should have all host components emit spans when they are called at the debug level. They may emit further spans at the trace level. Each host component should also emit any relevant metrics e.g. count of times called.

#### Trigger observability

All existing triggers should emit spans at the info level before routing to a specific component. They should also emit an info span before executing the Wasm. They should also emit any relevant metrics e.g. count of times called.

#### Implementation details

The Rust [tracing](https://docs.rs/tracing/latest/tracing/) library is used throughout Spin for emitting logs. This SIP suggests that [tracing_opentelemetry](https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry/) be used to emit the necessary spans throughout Spin. Special care will have to be taken to ensure that the spans in the tracing do not interfere with the logs Spin produces.

### Adding observability in the future

Improving the observability of Spin is an ongoing process. As new features are added to Spin some of them will require new observability to be added. The following is non-exhaustive list of possible new features that would require new observability to be added:

- **New triggers native to Spin**. An info span should be emitted before the trigger routes to a specific component. An info span should be emitted before executing the Wasm. Relevant metrics should be emitted.
- **New plugin triggers**. An info span should be emitted before the trigger routes to a specific component. An info span should be emitted before executing the Wasm. Relevant metrics should be emitted.
- **New host components**. A debug span should be emitted when the host component is called. Relevant metrics should be emitted.
- **New functionality in the Spin runtime**. This should be reserved for functionality that is critical to debugging the state of the Spin runtime. Debug spans should be added where relevant. Relevant metrics should be emitted.

Any new observability that is added to Spin should follow a consistent pattern. Defining this pattern is outside the scope of this SIP so we will defer to OTEL best practices. For guidance on naming spans see [here](https://github.com/open-telemetry/opentelemetry-specification/blob/v1.26.0/specification/trace/api.md#span). For other guidance on other semantic conventions see [here](https://opentelemetry.io/docs/concepts/semantic-conventions/).
