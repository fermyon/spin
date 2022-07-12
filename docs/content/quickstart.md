title = "Taking Spin for a spin"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/quickstart.md"
---

> This is an early preview of the Spin project. It is still experimental code,
> and you should expect breaking changes before the first stable release.

<iframe width="560" height="315" src="https://www.youtube.com/embed/sDiQV5RHorE" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

## Getting the `spin` binary

You can download the [latest release](https://github.com/fermyon/spin/releases).
For example, for an Apple silicon macOS machine:

```
$ wget https://github.com/fermyon/spin/releases/download/v0.4.0/spin-v0.4.0-macos-aarch64.tar.gz
$ tar xfv spin-v0.4.0-macos-aarch64.tar.gz
$ ./spin --help
```

If you have [`cargo`](https://doc.rust-lang.org/cargo/getting-started/installation.html), you can clone the repo and install it to your path:

```bash
$ git clone https://github.com/fermyon/spin -b v0.4.0
$ cd spin
$ rustup target add wasm32-wasi
$ cargo install --path .
$ spin --help
```

Alternatively, [follow the contribution document](./contributing.md) for a detailed guide
on building Spin from source:

```bash
$ git clone https://github.com/fermyon/spin
$ cd spin && make build
$ ./target/release/spin --help
```

At this point, move the `spin` binary somewhere in your path, so it can be
accessed from any directory.

### Linux: Additional Libraries

On a fresh Linux installation, you will also need the standard build toolchain
(`gcc`, `make`, etc.), the SSL library headers, and on some distributions you
may need `pkg-config`.

On Debian-like distributions, including Ubuntu, you can install these with a
command like this:

```console
$ sudo apt-get install build-essential libssl-dev pkg-config
```

## Creating a new Spin application from a template

Spin helps you create a new application based on templates:

```console
$ spin templates list
You have no templates installed. Run
spin templates install --git https://github.com/fermyon/spin
to install a starter set.
```

We first need to configure the [templates from the Spin repository](https://github.com/fermyon/spin/tree/main/templates):

```console
$ spin templates install --git https://github.com/fermyon/spin
Copying remote template source
Installing template redis-rust...
Installing template http-rust...
Installing template http-go...
+--------------------------------------------------+
| Name         Description                         |
+==================================================+
| http-go      HTTP request handler using (Tiny)Go |
| http-rust    HTTP request handler using Rust     |
| redis-rust   Redis message handler using Rust    |
| ...                                              |
+--------------------------------------------------+
```

> The Spin templates experience is still early — if you are interested in
> writing your own templates, you can follow the existing
> [templates from the Spin repository](https://github.com/fermyon/spin/tree/main/templates)
> and the [Spin Improvement Proposal (SIP) for templates](https://github.com/fermyon/spin/pull/273).

Let's create a new Spin application based on the Rust HTTP template:

```console
$ spin new http-rust spin-hello-world
Project description: A simple Spin HTTP component in Rust
HTTP base: /
HTTP path: /hello
$ tree
├── .cargo
│   └── config.toml
├── .gitignore
├── Cargo.toml
├── spin.toml
└── src
    └── lib.rs
```

This command created all the necessary files we need to build and run our first
Spin application. Here is `spin.toml`, the manifest file for a Spin application:

```toml
spin_version = "1"
description = "A simple Spin HTTP component in Rust"
name = "spin-hello-world"
trigger = { type = "http", base = "/" }
version = "0.1.0"

[[component]]
id = "spin-hello-world"
source = "target/wasm32-wasi/release/spin_hello_world.wasm"
[component.trigger]
route = "/hello"
[component.build]
command = "cargo build --target wasm32-wasi --release"
```

This represents a simple Spin HTTP application (triggered by an HTTP request), with
a single component called `spin-hello-world`. Spin will execute the `spin_hello_world.wasm`
WebAssembly module for HTTP requests on the route `/hello`.
(See the [configuration document](./configuration.md) for a detailed guide on the Spin
application manifest.)

Now let's have a look at the code. Below is the complete source
code for a Spin HTTP component written in Rust — a regular Rust function that
takes an HTTP request as a parameter and returns an HTTP response, and it is
annotated with the `http_component` macro:

```rust
use anyhow::Result;
use spin_sdk::{
    http::{Request, Response},
    http_component,
};

/// A simple Spin HTTP component.
#[http_component]
fn spin_hello_world(req: Request) -> Result<Response> {
    println!("{:?}", req.headers());
    Ok(http::Response::builder()
        .status(200)
        .header("foo", "bar")
        .body(Some("Hello, Fermyon".into()))?)
}
```

> See the document on writing [Rust](./rust-components.md) and [Go](./go-components.md)
> components for Spin, to ensure you have all dependencies installed.

For Rust templates you need the `wasm32-wasi` target. You can add it using `rustup`:`rustup target add wasm32-wasi`.

For TinyGo templates you need the [TinyGo toolchain installed](https://tinygo.org/getting-started/install/).

We can build this component using the regular Rust toolchain, targeting
`wasm32-wasi`, which will produce the WebAssembly module and place it at
`target/wasm32-wasi/release/spinhelloworld.wasm` as referenced in the
`spin.toml`. For convenience, we can use the `spin build` command, which will
execute the command defined above in `spin.toml` and call the Rust toolchain:

```console
$ spin build
Executing the build command for component spin-hello-world: cargo build --target wasm32-wasi --release
   Compiling spin_hello_world v0.1.0
    Finished release [optimized] target(s) in 0.10s
Successfully ran the build command for the Spin components.
```

> `spin build` can be used to build all components defined in the Spin manifest
> file at the same time, and also has a flag that starts the application after
> finishing the compilation, `spin build --up`.
>
> For more details, see the [page about developing Spin applications](./developing.md).

## Running the application with `spin up`

Now that we configured the application and built our component, we can _spin up_
the application (pun intended):

```bash
$ spin up
Serving HTTP on address http://127.0.0.1:3000
Available Routes:
  spin-hello-world: http://127.0.0.1:3000/hello
```

Optionally, set the RUST_LOG environment variable for detailed logs, before running `spin up`.

```bash
$ export RUST_LOG=spin=trace
```

Spin will instantiate all components from the application manifest, and
will create the router configuration for the HTTP trigger accordingly. The
component can now be invoked by making requests to `http://localhost:3000/hello`
(see route field in the configuration):

```
$ curl -i localhost:3000/hello
HTTP/1.1 200 OK
foo: bar
content-length: 15

Hello, Fermyon!
```

You can add as many components as needed in `spin.toml`, mount files and
directories, allow granular outbound HTTP connections, or set environment variables
(see the [configuration document](./configuration.md) for a detailed guide on
the Spin application manifest) and iterate locally with
`spin up --file spin.toml` until you are ready to distribute the application.

Congratulations! You just completed building and running your first Spin
application!
Next, check out the [Rust](./rust-components.md) or [Go](./go-components.md) language
guides, or have a look at [a more complex Spin application with components built
in multiple programming languages](https://github.com/fermyon/spin-kitchensink/).
