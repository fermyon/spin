title = "Taking Spin for a spin"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/quickstart.md"
---

## Getting the `spin` binary

<!-- You can download the [latest release](https://github.com/fermyon/spin/releases).
For example, for an M1 macOS machine:

```
$ wget https://github.com/fermyon/spin/releases/download/canary/spin-canary-macos-aarch64.tar.gz
$ tar xfv spin-canary-macos-aarch64.tar.gz
$ ./spin --help
```

> On an M1 macOS machine you might need to install / configure OpenSSL@1.1 by
> running
> `brew install openssl@1.1 && sudo ln -s /opt/homebrew/Cellar/openssl@1.1/1.1.1m /usr/local/openssl-aarch64` -->

First, [follow the contribution document](/contributing) for a detailed guide
on building Spin from source:

```bash
$ git clone https://github.com/fermyon/spin
$ cd spin && make build
$ ./target/release/spin --help
```

At this point, move the `spin` binary somewhere in your path, so it can be
accessed from any directory.

<!-- ## Creating a new Spin HTTP application in Rust

First, we need to add the official Spin templates from the repository:

```
$ spin templates add --git https://github.com/fermyon/spin --name fermyon
$ spin templates list
+-----------------------------------------------------------------------------------+
| Name        Repository   URL                                      Branch          |
+===================================================================================+
| spin-http     fermyon   https://github.com/fermyon/bartholomew   refs/heads/main |
+-----------------------------------------------------------------------------------+
```

Now we can create a new application from the template:

```
$ spin new --repo fermyon --template spin-http --path spin-hello-world
$ cd spin-hello-world
``` -->

## Building the example applications

Let's explore the Rust example from the `examples/http-rust` directory,
focusing first on `spin.toml`:

```toml
spin_version = "1"
name = "spin-hello-world"
trigger = { type = "http", base = "/" }
version = "1.0.0"

[[component]]
id = "hello"
source = "target/wasm32-wasi/release/spinhelloworld.wasm"
[component.trigger]
route = "/hello"
```

This is a simple Spin HTTP application (triggered by an HTTP request), with a
single component called `hello`. Spin will execute the `spinhelloworld.wasm`
WebAssembly module for HTTP requests on the route `/hello`.
(See the [configuration document](/configuration) for a detailed guide on the Spin
application configuration.)

Now let's have a look at the `hello` component. Below is the complete source
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
    Ok(http::Response::builder()
        .status(200)
        .header("foo", "bar")
        .body(Some("Hello, Fermyon!".into()))?)
}
```

> See [the section on building HTTP applications with Spin for a detailed guide](/writing-http-apps).

We can build this component using the regular Rust toolchain, targeting
`wasm32-wasi`, which will produce the WebAssembly module referenced in
`spin.toml`:

```
$ cargo build --target wasm32-wasi --release
```

## Running the application with `spin up`

Now that we configured the application and built our component, we can _spin up_
the application (pun intended):

```bash
# optionally, set the RUST_LOG environment variable for detailed logs 
$ export RUST_LOG=spin=trace
$ spin up --file spin.toml
INFO spin_http_engine: Serving HTTP on address 127.0.0.1:3000
```

Spin will instantiate all components from the application configuration, and
will crate the router configuration for the HTTP trigger accordingly. The
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
the Spin application configuration) and iterate locally with
`spin up --file spin.toml` until you are ready to distribute the application.

Congratulations! You just completed building and running your first Spin
application!
Next, check out the [Rust](/rust-components) or [Go](/go-components) language
guides.
