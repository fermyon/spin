<div align="center">
  <h1>Fermyon Spin</h1>
  <img src="./docs/static/image/logo.png" width="300"/>
  <p>Spin is a framework for building, deploying, and running fast, secure, and composable cloud microservices with WebAssembly.</p>
      <a href="https://github.com/fermyon/spin/actions/workflows/build.yml"><img src="https://github.com/fermyon/spin/actions/workflows/build.yml/badge.svg" alt="build status" /></a>
      <a href="https://discord.gg/eGN8saYqCk"><img alt="Discord" src="https://img.shields.io/discord/926888690310053918?label=Discord"></a>
</div>

> This is an early preview of the Spin project. It is still experimental code,
> and you should expect breaking changes before the first stable release.

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
[Rust](https://developer.fermyon.com/spin/rust-components/) or [Go](https://developer.fermyon.com/spin/go-components/)
language guides, and the [guide on configuring Spin applications](https://developer.fermyon.com/spin/configuration/).

After you build your application, run it using Spin:

```
$ spin up
```

## Contributing

We are delighted that you are interested in making Spin better! Thank you!
Please follow the [contributing guide](https://developer.fermyon.com/spin/contributing).
And join our [Discord server](https://discord.gg/eGN8saYqCk).

## Developer Meetings

Join the Spin monthly developer meetings, which will be announced in our [Discord server](https://discord.gg/eGN8saYqCk).
