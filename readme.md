<div align="center">
  <h1>Spin</h1>
  <img src="./docs/images/spin.png" width="300"/>
  <p>Spin is a framework for building, deploying, and running fast, secure, and composable cloud microservices with WebAssembly.</p>
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

See the [quickstart document](./docs/quickstart.md) for a detailed guide on
configuring Spin and writing your first Spin application, but in short:

```
$ wget https://github.com/fermyon/spin/releases/download/canary/spin-canary-<os-arch>.tar.gz
$ tar xfv spin-canary-<os-arch>.tar.gz
$ ./spin --help
```

After you follow the [quickstart document](./docs/quickstart.md), you can follow
the [guide on writing HTTP applications with Spin](./docs/writing-http-apps.md)
and the [guide on configuring Spin applications](./docs/configuration.md).

After you built your application, run it using Spin, pointing to the Spin
application configuration file:

```
$ spin up --file spin.toml
```

## Contributing

We are delighted that you are interested in making Spin better! Thank you!
Please follow the [contributing guide](./docs/contributing.md).
