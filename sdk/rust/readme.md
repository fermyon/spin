# The Spin Rust SDK

The Spin Rust SDK makes it easy to build Spin components in Rust.

### Writing Spin HTTP components in Rust

This library simplifies writing Spin HTTP components. Below is an example of
such a component:

```rust
// lib.rs
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

The important things to note in the function above:

- the `spin_sdk::http_component` macro — this marks the function as the
  entrypoint for the Spin component
- the function signature — `fn hello_world(req: Request) -> Result<Response>` —
  the Spin HTTP component uses the HTTP objects from the popular Rust crate
  [`http`](https://crates.io/crates/http), and the request and response bodies
  are optionally using [`bytes::Bytes`](https://crates.io/crates/bytes)

### Making outbound HTTP requests

This library includes the ability to send outbound HTTP requests using the
[DeisLabs WASI experimental HTTP library](https://github.com/deislabs/wasi-experimental-http).

Let's see an example where the component makes an outbound HTTP request to a
server, modifies the result, then returns it:

```rust
#[http_component]
fn hello_world(_req: Request) -> Result<Response> {
    let mut res = spin_sdk::http::send(
        http::Request::builder()
            .method("GET")
            .uri("https://fermyon.com")
            .body(None)?,
    )?;

    res.headers_mut()
        .insert(http::header::SERVER, "spin/0.1.0".try_into()?);

    Ok(res)
}

```

In order for the component above to be allowed to make the outbound HTTP
request, the destination host must be declared in the Spin application
configuration:

```toml
[[component]]
id = "hello"
source = "target/wasm32-wasi/release/spinhelloworld.wasm"
allowed_http_hosts = [ "https://fermyon.com" ]
[component.trigger]
route = "/hello"
```

Making a request to this component, we can see the appended header, and that the
response contains the expected body:

```
$ curl -I localhost:3000/hello
HTTP/1.1 200 OK
content-length: 29350
content-type: text/html; charset=utf-8
server: spin/0.1.0
```
