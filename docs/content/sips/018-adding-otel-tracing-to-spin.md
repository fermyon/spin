title = "SIP 018 - Adding OTel tracing to Spin"
template = "main"
date = "2024-02-27T12:00:00Z"

---

Summary: How to configure OTel support in Spin and add tracing.

Owner: caleb.schoepp@fermyon.com

Created: February 27, 2024

Updated: April 17, 2024

## Background

[Observability](https://opentelemetry.io/docs/concepts/observability-primer/#what-is-observability) is critical for a great developer experience. Improving the observability of Spin can be broken down into four categories:

1. Runtime observability: Observing the Spin runtime itself e.g. spans and metrics around important parts of the Spin runtime like host components.
2. Trigger observability: Observing the requests made to Spin applications e.g. spans and metrics around the requests made to Spin applications.
3. Component observability: Observing the interaction between composed components e.g having Wasmtime auto-instrument Wasm component graphs to produce spans.
4. Guest observability: Observing code within the guest module e.g. building a host component for WASI Observe.

More detail on these categories can be found [here](https://github.com/fermyon/spin/issues/2293).

## Proposal

This SIP aims to improve the runtime and trigger observability of Spin by emitting tracing. First, it will suggest how OTel tracing in Spin can be configured. Second, it will present guidelines for tracing in Spin.

### Configuring OTel

Spin will conform to the configuration options defined by OTel [here](https://opentelemetry.io/docs/specs/otel/protocol/exporter/) and [here](https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/#general-sdk-configuration).

A user of Spin effectively turns on observability by setting the environment variable `OTEL_EXPORTER_OTLP_ENDPOINT`. This value should be the endpoint of an OTLP compliant collector. They may explicitly turn off observability by setting `OTEL_SDK_DISABLED`.

Under the hood Spin will use the Rust [tracing](https://docs.rs/tracing/0.1.40/tracing/) crate to handle the instrumentation. The user may set `SPIN_OTEL_TRACING_LEVEL` to configure the level of tracing they want to emit to OTel.

In the future we will want to support per-component configuration of observability. For example this might look like the following in a `spin.toml` manifest.

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

### Adding tracing

As a general rule of thumb we only want to trace behavior that:

- Is fallible.
- Is potentially slow.
- Is not tightly nested under a parent span.
- Is relevant to an end user.

In practice this means tracing most of the triggers and host components.

Where [OTel semantic conventions](https://opentelemetry.io/docs/concepts/semantic-conventions/) exist we should follow them as closely as possible. Where semantic conventions don't exist we should:

- Give the span a name in the form `spin_{crate}.{operation}`.
- Track any relevant arguments as attributes.
