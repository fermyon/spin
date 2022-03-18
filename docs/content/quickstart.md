title = "Taking Spin for a Spin"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
author = "Fermyon"

---

# Taking Spin for a Spin

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

First, [follow the contribution guide](./contributing.md) for a detailed guide
on getting building Spin from source:

```shell
$ git clone https://github.com/fermyon/spin
$ cd spin && cargo build --release
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

Now let's look at the example applications from the `examples/` directory.

## Building the example applications

Let's first look at the Rust example from the `examples/http-rust` directory.  
Let's have a look at `spin.toml`:

```toml
spin_version = "1"
name = "spin-hello-world"
description = "A simple application that returns hello world."
trigger = { type = "http", base = "/" }
version = "1.0.0"

[[component]]
id = "hello"
source = "target/wasm32-wasi/release/spinhelloworld.wasm"
[component.trigger]
route = "/hello"
```

Since this is an HTTP application, the application trigger is of `type = http`,
and there is one component that responds to requests on route `/hello` using the
`spinhelloworld.wasm` WebAssembly module. (See the
[configuration document](./configuration.md) for a detailed guide on the Spin
application configuration.)

Now let's have a look at the `hello` component — below is the complete source
code for a Spin HTTP component written in Rust. It is a regular Rust function
that takes an HTTP request and returns an HTTP response, annotated with the
`http_component` macro:

```rust
use anyhow::Result;
use spin_sdk::{
    http::{Request, Response},
    http_component,
};

/// A simple Spin HTTP component.
#[http_component]
fn hello_world(req: Request) -> Result<Response> {
    println!("{:?}", req.headers());
    Ok(http::Response::builder()
        .status(200)
        .header("foo", "bar")
        .body(Some("Hello, Fermyon!".into()))?)
}
```

> See
> [the section on building HTTP applications with Spin for a detailed guide](./writing-http-apps.md).

We can build this component using the regular Rust toolchain, targeting
`wasm32-wasi`, which will produce the WebAssembly module referenced in
`spin.toml`:

```
$ cargo build --target wasm32-wasi --release
```

## Running the application with `spin up`

Now that we configured the application and built our component, we can _spin up_
the application (pun intended):

```shell
# optionally, use RUST_LOG=spin=trace to see detailed logs
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
directories, allow granular outbound HTTP connections, or environment variables.
(see the [configuration document](./configuration.md) for a detailed guide on
the Spin application configuration) and iterate locally with
`spin up --file spin.toml` until you are ready to distribute the application.

## Distributing the application

First, we need to start the registry. You can
[install the latest Bindle release](https://github.com/deislabs/bindle/tree/main/docs#from-the-binary-releases),
or use the
[`autobindle`](https://marketplace.visualstudio.com/items?itemName=fermyon.autobindle)
VS Code extension, which automatically downloads and starts Bindle on
`http://localhost:8080/v1`. Now we can package the entire application, the
components, and all the referenced files and publishes them to the registry:

```
$ export BINDLE_URL=http://localhost:8080/v1
$ spin bindle push --file spin.toml
pushed: spin-hello-world/1.0.0
```

Now we can run the application using `spin up` directly from the registry:

```
$ spin up --bindle spin-hello-world/1.0.0
```

Congratulations! You just completed writing, building, publishing, and running
your first Spin application.
