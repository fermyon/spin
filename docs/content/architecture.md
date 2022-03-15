title = "Architecture and Internals"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
author = "Fermyon"

---

# Spin architecture and internals

This document aims to offer an overview to the implementation of Spin, as well
as explain how the code is structured and how all parts fit together. This
document is continuously evolving, and if you want even more detailed
information, make sure to review the code for a given part of Spin.

## How Spin runs an application

A Spin application is defined as a `spin.toml` file. It can either be run
directly by `spin up`, passing the manifest file (`--file spin.toml`), or it can
be pushed to the registry then referenced using its remote ID
(`spin bindle push` followed by `spin up --bindle <id>`).

Regardless of the application origin (local file or remote reference from the
registry), a Spin application is defined by
`spin_config::Configuration<CoreComponent>` (contained in the
[`spin-config`](../crates/config) crate), which is the canonical representation
of a Spin application.

The crate responsible for transforming a custom configuration into a canonical
Spin application is [`spin-loader`](../crates/loader), which implements loading
applications from local `spin.toml` files and from remote Bindle references (and
ensures files referenced in the application configuration are copied and mounted
at the location expected in the WebAssmebly module). Once the canonical
representation is loaded from an application source, it is passed to a trigger —
currently, the only trigger implemented is the HTTP trigger, and we will use it
as an example throughout this document.

The HTTP trigger (defined in the [`spin-http`](../crates/http) crate) takes an
application configuration ([#40](https://github.com/fermyon/spin/issues/40)
explores a trigger handling multiple applications), starts an HTTP listener, and
for each new request, it routes it to the component configured in the
application configuration. Then, it instantiates the WebAssembly module (using a
`spin_engine::ExecutionContext`) and uses the appropriate executor (either the
[`SpinHttpExecutor`](../crates/http/src/spin.rs) or the
[`WagiHttpExecutor`](../crates/http/src/wagi.rs), based on the component
configuration) to handle the request and return the response.

## The Spin execution context

The Spin execution context (or "Spin engine") is the part of Spin that executes
WebAssembly components using the
[Wasmtime](https://github.com/bytecodealliance/wasmtime) WebAssembly runtime. It
is implemented in the [`spin-engine`](../crates/engine/) crate, and serves as
the part of Spin that takes a fully formed application configuration and creates
Wasm instances based on the component configurations.

There are two important concepts in this crate:

- `spin_engine::Builder` — the builder for creating an execution context. It is
  created using an `ExecutionContextConfiguration` object (which contains a Spin
  application and Wasmtime configuration), and implements the logic for
  configuring WASI and the other host implementations provided by Spin. The
  builder exposes the Wasmtime
  [`Linker`](https://docs.rs/wasmtime/latest/wasmtime/struct.Linker.html),
  [`Engine`](https://docs.rs/wasmtime/latest/wasmtime/struct.Engine.html), and
  [`Store<RuntimeContext<T>>`](https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html)
  (where `RuntimeContext<T>` is the internal Spin context, which is detailed
  later in the document), and it uses them to [pre-instantiate]()

- `spin_engine::ExecutionContext` — the main execution engine in Spin.
