<div align="center">
  <h1>Fermyon Spin</h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./docs/static/image/logo-dark.png">
    <img alt="spin logo" src="./docs/static/image/logo.png" width="300" height="128">
  </picture>
  <p>Spin is a framework for building, deploying, and running fast, secure, and composable cloud microservices with WebAssembly.</p>
      <a href="https://github.com/fermyon/spin/actions/workflows/build.yml"><img src="https://github.com/fermyon/spin/actions/workflows/build.yml/badge.svg" alt="build status" /></a>
      <a href="https://discord.gg/eGN8saYqCk"><img alt="Discord" src="https://img.shields.io/discord/926888690310053918?label=Discord"></a>
</div>

## What is Spin?

Spin is an open source framework for building and running fast, secure, and
composable cloud microservices with WebAssembly. It aims to be the easiest way
to get started with WebAssembly microservices, and takes advantage of the latest
developments in the
[WebAssembly component model](https://github.com/WebAssembly/component-model)
and [Wasmtime](https://wasmtime.dev/) runtime.

Spin offers a simple CLI that helps you create, distribute, and execute
applications, and in the next sections we will learn more about Spin
applications and how to get started.

## Getting started

See the [quickstart document](https://developer.fermyon.com/spin/quickstart/) for a detailed
guide on configuring Spin and writing your first Spin application, but in short:

```
$ wget https://github.com/fermyon/spin/releases/download/<version>/spin-<version>-<os-arch>.tar.gz
$ tar xfv spin-<version>-<os-arch>.tar.gz
$ ./spin --help
```

> Alternatively, you could [build Spin from source](https://developer.fermyon.com/spin/contributing/).

After you follow the [quickstart document](https://developer.fermyon.com/spin/quickstart/),
you can follow the
[Rust](https://developer.fermyon.com/spin/rust-components/), [JavaScript](https://developer.fermyon.com/spin/javascript-components), [Python](https://developer.fermyon.com/spin/python-components), or [Go](https://developer.fermyon.com/spin/go-components/)
language guides, and the [guide on configuring Spin applications](https://developer.fermyon.com/spin/configuration/).

Below is an example of using the `spin` CLI to create a new Spin Python application, then adding a JavaScript component:

```bash
# Create a new Spin application based on the Python language template.
$ spin new http-py hello-python
# Add a new JavaScript component based on the language template.
$ spin add http-js goodbye-javascript
```

Running the `spin add` command will generate the proper configuration for our component and add it to the [`spin.toml` manifest file](https://developer.fermyon.com/spin/manifest-reference). For example, here is the `spin.toml` section for our Python component:

```toml
[[component]]
# The ID of the component.
id = "hello-python"
# The Wasm module to instantiate and execute when receiving a request.
source = "hello-python/app.wasm"
[component.trigger]
# The route for this component.
route = "/hello"
[component.build]
# The command to execute for this component with `spin build`.
command = "spin py2wasm app -o app.wasm"
# The working directory for the component.
workdir = "hello-python"
```

We can now build our application with `spin build`, then run it locally with `spin up`:

```bash
# Compile all components to Wasm by executing their `build` commands.
$ spin build
Executing the build command for component hello-python: spin py2wasm app -o app.wasm
Executing the build command for component goodbye-javascript: npm run build
Successfully ran the build command for the Spin components.
# Run the application locally.
$ spin up
Logging component stdio to ".spin/logs/"
Serving http://127.0.0.1:3000
Available Routes:
  hello-python: http://127.0.0.1:3000/hello
  goodbye-javascript: http://127.0.0.1:3000/goodbye
```

Once the application is running, we can start testing it by sending requests to its components:

```bash
# Send a request to the Python component.
$ curl localhost:3000/hello
Hello, Python!
# Send a request to the JavaScript component.
$ curl localhost:3000/goodbye
Goodbye, JavaScript!
```

When handling a request, Spin will create a new isolated Wasm instance corresponding to the Wasm module for the matching component, execute the handler function, then terminate the instance. Each new request will get a fresh Wasm instance.

## Language Support for Spin Features

| Feature | Rust SDK Supported? | TypeScript SDK Supported? | Python SDK Supported? | Tiny Go SDK Supported? | C# SDK Supported? |
|-----|-----|-----|-----|-----|-----|
| **Triggers** |
| [HTTP](https://developer.fermyon.com/spin/http-trigger) | Supported | Supported | Supported | Supported | Supported |
| [Redis](https://developer.fermyon.com/spin/redis-trigger) | Supported | Not Supported | Not Supported | Supported | Not Supported |
| **APIs** |
| [Outbound HTTP](https://developer.fermyon.com/spin/rust-components.md#sending-outbound-http-requests) | Supported | Supported | Supported | Supported | Supported |
| [Key Value Storage](https://developer.fermyon.com/spin/kv-store.md) | Supported | Supported | Supported | Supported | Not Supported |
| [MySQL](https://developer.fermyon.com/spin/rdbms-storage#using-mysql-and-postgresql-from-applications) | Supported | Not Supported | Not Supported | Not Supported | Not Supported |
| [PostgreSQL](https://developer.fermyon.com/spin/rdbms-storage#using-mysql-and-postgresql-from-applications) | Supported | Not Supported | Not Supported | Not Supported | Supported |
| [Outbound Redis](https://developer.fermyon.com/spin/rust-components.md#storing-data-in-redis-from-rust-components) | Supported | Supported | Supported | Supported | Supported |
| **Extensibility** |
| [Authoring Custom Triggers](https://developer.fermyon.com/spin/extending-and-embedding) | Supported | Not Supported | Not Supported | Not Supported | 




## Contributing

We are delighted that you are interested in making Spin better! Thank you!
Please follow the [contributing guide](https://developer.fermyon.com/spin/contributing).
And join our [Discord server](https://discord.gg/eGN8saYqCk).

## Stay in Touch

Join the Spin community in our [Discord server](https://discord.gg/eGN8saYqCk).
