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

See the [Install Spin](https://developer.fermyon.com/spin/install) page of the [Spin documentation](https://developer.fermyon.com/spin/index) for a detailed
guide on installing and configuring Spin, but in short run the following commands:
```bash
curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash
sudo mv ./spin /usr/local/bin/spin
```

Alternatively, you could [build Spin from source](https://developer.fermyon.com/spin/contributing/).

To get started writing apps, follow the [quickstart guide](https://developer.fermyon.com/spin/quickstart/),
and then follow the
[Rust](https://developer.fermyon.com/spin/rust-components/), [JavaScript](https://developer.fermyon.com/spin/javascript-components), [Python](https://developer.fermyon.com/spin/python-components), or [Go](https://developer.fermyon.com/spin/go-components/)
language guides, and the [guide on writing Spin applications](https://developer.fermyon.com/spin/configuration/).

## Usage
Below is an example of using the `spin` CLI to create a new Spin application.  To run the example you will need to install the `wasm32-wasi` target for Rust.

```bash
$ rustup target add wasm32-wasi
```

First, run the `spin new` command to create a Spin application from a template.
```bash
# Create a new Spin application named 'hello-rust' based on the Rust http template, accepting all defaults
$ spin new --accept-defaults http-rust hello-rust
```
Running the `spin new` command created a `hello-rust` directory with all the necessary files for your application. Change to the `hello-rust` directory and build the application with `spin build`, then run it locally with `spin up`:

```bash
# Compile to Wasm by executing the `build` command.
$ spin build
Executing the build command for component hello-rust: cargo build --target wasm32-wasi --release
    Finished release [optimized] target(s) in 0.03s
Successfully ran the build command for the Spin components.

# Run the application locally.
$ spin up
Logging component stdio to ".spin/logs/"

Serving http://127.0.0.1:3000
Available Routes:
  hello-rust: http://127.0.0.1:3000 (wildcard)
```

That's it! Now that the application is running, use your browser or cURL in another shell to try it out:

```bash
# Send a request to the application.
$ curl -i 127.0.0.1:3000
HTTP/1.1 200 OK
foo: bar
content-length: 14
date: Thu, 13 Apr 2023 17:47:24 GMT

Hello, Fermyon         
```
You can make the app do more by editting the `src/lib.rs` file in the `hello-rust` directory using your favorite editor or IDE. To learn more about writing Spin applications see [Writing Applications](https://developer.fermyon.com/spin/writing-apps) in the Spin documentation.  To learn how to publish and distribute your application see the [Publishing and Distribution](https://developer.fermyon.com/spin/distributing-apps) guide in the Spin documentation.

For more information on the cli commands and subcommands see the [CLI Reference](https://developer.fermyon.com/common/cli-reference).

## Language Support for Spin Features

The table below summarizes the [feature support](https://developer.fermyon.com/spin/language-support-overview) in each of the language SDKs.

| Feature | Rust SDK Supported? | TypeScript SDK Supported? | Python SDK Supported? | Tiny Go SDK Supported? | C# SDK Supported? |
|-----|-----|-----|-----|-----|-----|
| **Triggers** |
| [HTTP](https://developer.fermyon.com/spin/http-trigger) | Supported | Supported | Supported | Supported | Supported |
| [Redis](https://developer.fermyon.com/spin/redis-trigger) | Supported | Not Supported | Not Supported | Supported | Not Supported |
| **APIs** |
| [Outbound HTTP](https://developer.fermyon.com/spin/rust-components.md#sending-outbound-http-requests) | Supported | Supported | Supported | Supported | Supported |
| [Configuration Variables](https://developer.fermyon.com/spin/variables) | Supported | Supported | Supported | Supported | Supported |
| [Key Value Storage](https://developer.fermyon.com/spin/kv-store-api-guide) | Supported | Supported | Supported | Supported | Not Supported |
| [Sqlite Storage](https://developer.fermyon.com/spin/sqlite-api-guide) | Supported | Supported | Supported | Not Supported | Not Supported |
| [MySQL](https://developer.fermyon.com/spin/rdbms-storage#using-mysql-and-postgresql-from-applications) | Supported | Not Supported | Not Supported | Not Supported | Not Supported |
| [PostgreSQL](https://developer.fermyon.com/spin/rdbms-storage#using-mysql-and-postgresql-from-applications) | Supported | Not Supported | Not Supported | Not Supported | Supported |
| [Outbound Redis](https://developer.fermyon.com/spin/rust-components.md#storing-data-in-redis-from-rust-components) | Supported | Supported | Supported | Supported | Supported |
| [Serverless AI](https://developer.fermyon.com/spin/rust-components#ai-inferencing-from-rust-components) | Supported | Supported | Not Supported | Not Supported | Not Supported |
| **Extensibility** |
| [Authoring Custom Triggers](https://developer.fermyon.com/spin/extending-and-embedding) | Supported | Not Supported | Not Supported | Not Supported | Not Supported |

## Contributing

We are delighted that you are interested in making Spin better! Thank you!
Please follow the [contributing guide](https://developer.fermyon.com/spin/contributing).
And join our [Discord server](https://discord.gg/eGN8saYqCk).

## Stay in Touch
Follow us on Twitter: [@spinframework](https://twitter.com/spinframework)

You can join the Spin community in our [Discord server](https://discord.gg/eGN8saYqCk) where you can ask questions, get help, and show off the cool things you are doing with Spin!

