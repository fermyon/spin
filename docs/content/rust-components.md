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

Besides `cargo` and the Rust compiler, you need to add the `wasm32-wasi` target
before compiling Rust components for Spin:

In order to compile Rust programs to Spin components, you also need the
`wasm32-wasi` target. You can add it using `rustup`:

```plaintext
$ rustup target add wasm32-wasi
```

## HTTP components

In Spin, HTTP components are triggered by the occurrence of an HTTP request, and
must return an HTTP response at the end of their execution. Components can be
built in any language that compiles to WASI, and Rust has improved support
for writing applications, through its SDK.

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

If allowed, Spin components can send outbound HTTP requests using the [DeisLabs
WASI experimental HTTP library](https://github.com/deislabs/wasi-experimental-http).
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

We just built a WebAssembly component that sends an HTTP request to another
service, manipulates that result, then responds to the original request.
This can be the basis for building components that communicate with external
databases or storage accounts, or even more specialized components like HTTP
proxies or URL shorteners.

## Redis components

Besides the HTTP trigger, Spin has built-in support for a Redis trigger —
which will connect to a Redis instance and will execute Spin components for
new messages on the configured channels.

> See the [Redis trigger](/redis-trigger) for details about the Redis trigger.

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

> We are also evaluating adding
> [host support for connecting to Redis databases](https://github.com/fermyon/spin/issues/181),
> which would allow using the key/value store and publishing messages to channels.

## Using external crates in Rust components

In Rust, Spin components are regular libraries that contain a function
annotated using the `http_component` macro, compiled to the
[`wasm32-wasi` target](https://doc.rust-lang.org/stable/nightly-rustc/rustc_target/spec/wasm32_wasi/index.html).
This means that any [crate](https://crates.io) that compiles to `wasm32-wasi` can
be used when implementing the component.

> Make sure to read [the page describing the HTTP trigger](/http-trigger) for more
> details about building HTTP applications.

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
