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
For example, for an M1 macOS machine:

```
$ wget https://github.com/fermyon/spin/releases/download/v0.1.0/spin-v0.1.0-macos-aarch64.tar.gz
$ tar xfv spin-v0.1.0-macos-aarch64.tar.gz
$ ./spin --help
```

Alternatively, [follow the contribution document](/contributing) for a detailed guide
on building Spin from source:

```bash
$ git clone https://github.com/fermyon/spin
$ cd spin && make build
$ ./target/release/spin --help
```

At this point, move the `spin` binary somewhere in your path, so it can be
accessed from any directory.

## Building the example applications

To build and run the Spin example applications, clone the Spin repository:

```
$ git clone https://github.com/fermyon/spin
```

Let's explore [the Rust example from the `examples/http-rust` directory](https://github.com/fermyon/spin/tree/main/examples/http-rust):

```
$ cd examples/http-rust
```

Here is the `spin.toml`, the definition file for a Spin application:

```toml
spin_version = "1"
name = "spin-hello-world"
version = "1.0.0"
trigger = { type = "http", base = "/" }

[[component]]
id = "hello"
source = "target/wasm32-wasi/release/spinhelloworld.wasm"
[component.trigger]
route = "/hello"
```

This represents a simple Spin HTTP application (triggered by an HTTP request), with
a single component called `hello`. Spin will execute the `spinhelloworld.wasm`
WebAssembly module for HTTP requests on the route `/hello`.
(See the [configuration document](/configuration) for a detailed guide on the Spin
application manifest.)

Now let's have a look at the `hello` component (`examples/http-rust/src/lib.rs`). Below is the complete source
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
fn hello_world(req: Request) -> Result<Response> {
    println!("{:?}", req);
    Ok(http::Response::builder()
        .status(200)
        .header("foo", "bar")
        .body(Some("Hello, Fermyon!".into()))?)
}
```

> See the document on writing [Rust](/rust-components) and [Go](/go-components)
> components for Spin.

We can build this component using the regular Rust toolchain, targeting
`wasm32-wasi`, which will produce the WebAssembly module and place it at
`target/wasm32-wasi/release/spinhelloworld.wasm` as referenced in the
`spin.toml`:

```
$ cargo build --release
```

> We are [working on templates](https://github.com/fermyon/spin/pull/186)
> to streamline the process of creating new applications.

## Running the application with `spin up`

Now that we configured the application and built our component, we can _spin up_
the application (pun intended):

```bash
# optionally, set the RUST_LOG environment variable for detailed logs
$ export RUST_LOG=spin=trace
# `spin up` in the example/http-rust directory
# alternatively, use `spin up --file <path/to/spin.toml>`
$ spin up
INFO spin_http_engine: Serving HTTP on address 127.0.0.1:3000
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
(see the [configuration document](/configuration) for a detailed guide on
the Spin application manifest) and iterate locally with
`spin up --file spin.toml` until you are ready to distribute the application.

Congratulations! You just completed building and running your first Spin
application!
Next, check out the [Rust](/rust-components) or [Go](/go-components) language
guides, or have a look at [a more complex Spin application with components built
in multiple programming languages](https://github.com/fermyon/spin-kitchensink/).
