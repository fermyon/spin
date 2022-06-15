title = "Building Spin components in Rust"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/rust-components.md"
---

Spin aims to have best-in-class support for building components in Rust, and
writing such components should be familiar for Rust developers.

> This guide assumes you are familiar with the Rust programming language,
> but if you are just getting started, be sure to check [the
official resources for learning Rust](https://www.rust-lang.org/learn).

> All examples from this page can be found in [the Spin repository on GitHub](https://github.com/fermyon/spin/tree/main/examples).

In order to compile Rust programs to Spin components, you also need the
`wasm32-wasi` target. You can add it using `rustup`:

```console
$ rustup target add wasm32-wasi
```

## HTTP components

In Spin, HTTP components are triggered by the occurrence of an HTTP request, and
must return an HTTP response at the end of their execution. Components can be
built in any language that compiles to WASI, but Rust has improved support
for writing Spin components with the Spin Rust SDK.

> Make sure to read [the page describing the HTTP trigger](./http-trigger.md) for more
> details about building HTTP applications.

Building a Spin HTTP component using the Rust SDK means writing a single function
that takes an HTTP request as a parameter, and returns an HTTP response — below
is a complete implementation for such a component:

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

The important things to note in the implementation above:

- the `spin_sdk::http_component` macro marks the function as the
  entry point for the Spin component
- the function signature — `fn hello_world(req: Request) -> Result<Response>` —
  the Spin HTTP component uses the HTTP objects from the popular Rust crate
  [`http`](https://crates.io/crates/http), and the request and response bodies
  are optionally using [`bytes::Bytes`](https://crates.io/crates/bytes)
  (`spin_sdk::http::Request` is a type alias for `http::Request<Option<Bytes>>`)
- the component returns a Rust `anyhow::Result`, so if there is an error processing the request, it returns an `anyhow::Error`.

## Sending outbound HTTP requests

If allowed, Spin components can send outbound HTTP requests.
Let's see an example of a component that makes a request to
[an API that returns random dog facts](https://some-random-api.ml/facts/dog) and
inserts a custom header into the response before returning:

```rust
#[http_component]
fn hello_world(_req: Request) -> Result<Response> {
    let mut res = spin_sdk::http::send(
        http::Request::builder()
            .method("GET")
            .uri("https://some-random-api.ml/facts/dog")
            .body(None)?,
    )?;

    res.headers_mut()
        .insert(http::header::SERVER, "spin/0.1.0".try_into()?);

    Ok(res)
}
```

Before we can execute this component, we need to add the `https://some-random-api.ml`
domain to the application manifest `allowed_http_hosts` list containing the list of
domains the component is allowed to make HTTP requests to:

```toml
# spin.toml
spin_version = "1"
name = "spin-hello-world"
trigger = { type = "http", base = "/" }
version = "1.0.0"

[[component]]
id = "hello"
source = "target/wasm32-wasi/release/spinhelloworld.wasm"
allowed_http_hosts = [ "https://some-random-api.ml" ]
[component.trigger]
route = "/hello"
```

Running the application using `spin up --file spin.toml` will start the HTTP
listener locally (by default on `localhost:3000`), and our component can
now receive requests in route `/hello`:

```bash
$ curl -i localhost:3000/hello
HTTP/1.1 200 OK
date: Fri, 18 Mar 2022 03:54:36 GMT
content-type: application/json; charset=utf-8
content-length: 185
server: spin/0.1.0

{"fact":"It's rumored that, at the end of the Beatles song, 
\"A Day in the Life,\" Paul McCartney recorded an ultrasonic whistle, 
audible only to dogs, just for his Shetland sheepdog."}
```

> Without the `allowed_http_hosts` field populated properly in `spin.toml`,
> the component would not be allowed to send HTTP requests, and sending the
> request would result in a "Destination not allowed" error.

> You can set `allowed_http_hosts = ["insecure:allow-all"]` if you want to allow
> the component to make requests to any HTTP host. This is **NOT** recommended
> for any production or publicly-accessible application.

We just built a WebAssembly component that sends an HTTP request to another
service, manipulates that result, then responds to the original request.
This can be the basis for building components that communicate with external
databases or storage accounts, or even more specialized components like HTTP
proxies or URL shorteners.

## Redis components

Besides the HTTP trigger, Spin has built-in support for a Redis trigger —
which will connect to a Redis instance and will execute Spin components for
new messages on the configured channels.

> See the [Redis trigger](./redis-trigger.md) for details about the Redis trigger.

Writing a Redis component in Rust also takes advantage of the SDK:

```rust
/// A simple Spin Redis component.
#[redis_component]
fn on_message(msg: Bytes) -> Result<()> {
    println!("{}", from_utf8(&msg)?);
    Ok(())
}
```

- the `spin_sdk::redis_component` macro marks the function as the
  entry point for the Spin component
- in the function signature — `fn on_message(msg: Bytes) -> anyhow::Result<()>` —
`msg` contains the payload from the Redis channel
- the component returns a Rust `anyhow::Result`, so if there is an error
processing the request, it returns an `anyhow::Error`.

The component can be built with Cargo by executing:

```bash
$ cargo build --target wasm32-wasi --release
```

The manifest for a Redis application must contain the address of the Redis
instance the trigger must connect to:

```toml
spin_version = "1"
name = "spin-redis"
trigger = { type = "redis", address = "redis://localhost:6379" }
version = "0.1.0"

[[component]]
id = "echo-message"
source = "target/wasm32-wasi/release/spinredis.wasm"
[component.trigger]
channel = "messages"
```

This application will connect to `redis://localhost:6379`, and for every new
message on the `messages` channel, the `echo-message` component will be executed.

```bash
# first, start redis-server on the default port 6379
$ redis-server --port 6379
# then, start the Spin application
$ spin up --file spin.toml
INFO spin_redis_engine: Connecting to Redis server at redis://localhost:6379
INFO spin_redis_engine: Subscribed component 0 (echo-message) to channel: messages
```

For every new message on the  `messages` channel:

```bash
$ redis-cli
127.0.0.1:6379> publish messages "Hello, there!"
```

Spin will instantiate and execute the component we just built:

```
INFO spin_redis_engine: Received message on channel "messages"
Hello, there!
```

> You can find a complete example for a Redis triggered component in the
> [Spin repository on GitHub](https://github.com/fermyon/spin/tree/main/examples/redis-rust).

## Storing data in Redis from Rust components

Using the Spin's Rust SDK, you can use the Redis key/value store and to publish
messages to Redis channels. This can be used from both HTTP and Redis triggered
components.

Let's see how we can use the Rust SDK to connect to Redis:

```rust
#[spin_sdk::http_component]
fn publish(_req: Request) -> Result<Response> {
    let address = std::env::var(REDIS_ADDRESS_ENV)?;
    let channel = std::env::var(REDIS_CHANNEL_ENV)?;

    // Get the message to publish from the Redis key "mykey"
    let payload = spin_sdk::redis::get(&address, &"mykey").map_err(|_| anyhow!("Error querying Redis"))?;

    // Set the Redis key "spin-example" to value "Eureka!"
    spin_sdk::redis::set(&address, &"spin-example", &b"Eureka!"[..])
        .map_err(|_| anyhow!("Error executing Redis command"))?;

    // Publish to Redis
    match spin_sdk::redis::publish(&address, &channel, &payload) {
        Ok(()) => Ok(http::Response::builder().status(200).body(None)?),
        Err(_e) => internal_server_error(),
    }
}
```

This HTTP component demonstrates fetching a value from Redis by key, setting a
key with a value, and publishing a message to a Redis channel. The component is
triggered by an HTTP request served on the route configured in the `spin.toml`:

```toml
[[component]]
environment = { REDIS_ADDRESS = "redis://127.0.0.1:6379", REDIS_CHANNEL = "messages" }
[component.trigger]
route = "/publish"
```

This HTTP component can be paired with a Redis component, triggered on new
messages on the `messages` Redis channel.

> You can find a complete example for using outbound Redis from an HTTP component
> in the [Spin repository on GitHub](https://github.com/fermyon/spin/tree/main/examples/rust-outbound-redis).

## Using external crates in Rust components

In Rust, Spin components are regular libraries that contain a function
annotated using the `http_component` macro, compiled to the
[`wasm32-wasi` target](https://doc.rust-lang.org/stable/nightly-rustc/rustc_target/spec/wasm32_wasi/index.html).
This means that any [crate](https://crates.io) that compiles to `wasm32-wasi` can
be used when implementing the component.

## Troubleshooting

Sometimes things can go wrong, especially such early projects. If you bump into
issues building and running your Rust component:

- ensure `cargo` is present in your path — we recommend
[Rust](https://www.rust-lang.org/) at [1.56+](https://www.rust-lang.org/tools/install)
- ensure `wasm32-wasi` target is configured for your Rust installation —
you can add it by running `rustup target add wasm32-wasi`
- build a `release` version of the component — all Spin application definitions
(`spin.toml` files) reference build configuration for Rust, so make sure to
run `cargo build --release --target wasm32-wasi` when building your components
(you can validate modules are correctly being built by checking the contents of
the `target/wasm32-wasi/release` directory and looking for `.wasm` files)
- make sure the path and name of the Wasm module in `target/wasm32-wasi/release`
match `source` field in the component configuration (the `source` field contains
the path to the Wasm module, relative to `spin.toml`)

## Manually creating new projects with Cargo

The recommended way of creating new Spin projects is by starting from a template.
This section shows how to  manually create a new project with Cargo.

When creating a new Spin projects with Cargo, you should use the `--lib` flag.

```console
$ cargo init --lib
```

A `Cargo.toml` with standard Spin dependencies looks like this:

```toml
[package]
name = "your-app"
version = "0.1.0"
edition = "2021"

[lib]
# Required to have a `cdylib` (dynamic library) to produce a Wasm module.
crate-type = [ "cdylib" ]

[dependencies]
# Useful crate to handle errors.
anyhow = "1"
# Crate to simplify working with bytes.
bytes = "1"
# General-purpose crate with common HTTP types.
http = "0.2"
# The Spin SDK.
spin-sdk = { git = "https://github.com/fermyon/spin" }
# Crate that generates Rust Wasm bindings from a WebAssembly interface.
wit-bindgen-rust = { git = "https://github.com/bytecodealliance/wit-bindgen", rev = "dde4694aaa6acf9370206527a798ac4ba6a8c5b8" }
```

At the time of this writing, `wit-bindgen` must be pinned to a specific `rev`.
This will change in the future.
