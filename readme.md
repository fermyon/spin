<div align="center">
  <h1>Spin</h1>
  <img src="./docs/static/image/spin.png" width="300"/>
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

See the [quickstart document](./docs/content/quickstart.md) for a detailed
guide on configuring Spin and writing your first Spin application, but in short:

```
$ wget https://github.com/fermyon/spin/releases/download/<version>/spin-<version>-<os-arch>.tar.gz
$ tar xfv spin-<version>-<os-arch>.tar.gz
$ ./spin --help
```

> Alternatively, you could [build Spin from source](./docs/content/contributing.md).

After you follow the [quickstart document](./docs/content/quickstart.md),
you can follow the
[guide on writing HTTP applications with Spin](./docs/content/writing-http-apps.md)
and the
[guide on configuring Spin applications](./docs/content/configuration.md).

After you built your application, run it using Spin, pointing to the Spin
application configuration file:

```
$ spin up --file spin.toml
```

## Contributing

We are delighted that you are interested in making Spin better! Thank you!
Please follow the [contributing guide](./docs/content/contributing.md).
