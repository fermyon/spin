title = "Introducing Spin"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
author = "Fermyon"

---

# Introducing Spin

Spin is an open source framework for building and running fast, secure, and
composable cloud microservices with WebAssembly. It aims to be the easiest way
to get started with WebAssembly microservices, and takes advantage of the latest
developments in the
[WebAssembly component model](https://github.com/WebAssembly/component-model)
and [Wasmtime](https://wasmtime.dev/) runtime.

Spin offers a simple CLI that helps you create, distribute, and execute
applications, and in the next sections we will learn more about Spin
applications and how to get started.

## Spin applications

Spin applications are comprised of one or more _components_, and follow the
event-driven model — they are executed as the result of events being generated
by _triggers_ (for example an HTTP server receiving requests, or a queue
subscription receiving messages). On each new event, _the entrypoint_ of a
component is executed by Spin. The entrypoints to components are _functions_.
This, together with the fact that they are invoked in response to events, brings
the Spin application model closer to the Function-as-a-Service model.

In the next section, we will [take Spin for a spin](./quickstart.md).
