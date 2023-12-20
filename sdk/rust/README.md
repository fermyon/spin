# The Spin Rust SDK

The Spin Rust SDK makes it easy to build Spin components in Rust.

## Fermyon Developer Home

This `README` file provides a few examples, such as writing Spin HTTP components in Rust and making outbound HTTP requests. For comprehensive information, visit the official [Fermyon Developer Home](https://developer.fermyon.com/). This resource includes [a page on installing Spin](https://developer.fermyon.com/spin/v2/install#installing-spin), [a quickstart guide](https://developer.fermyon.com/spin/v2/quickstart), and [a language support overview page](https://developer.fermyon.com/spin/v2/language-support-overview). The latter lists all of Spin's features—including key-value storage, SQLite, MySQL, Redis, Serverless AI, etc.—and their implementation in specific languages such as Rust, TS/JS, Python, and TinyGo.

### Writing Spin HTTP Components in Rust

This library simplifies writing Spin HTTP components. Below is an example of
such a component:

```rust
// lib.rs
use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;

/// A simple Spin HTTP component.
#[http_component]
fn handle_hello_world(req: Request) -> anyhow::Result<impl IntoResponse> {
    println!("Handling request to {:?}", req.header("spin-full-url"));
    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body("Hello, Fermyon")
        .build())
}
```

The important things to note about the function above are:

- the `spin_sdk::http_component` macro marks the function as the entry point for the Spin component,
- in the function signature (`fn handle_hello_world(req: Request) -> anyhow::Result<impl IntoResponse>`), `req` can be any number of types, including the built-in `Request` type or the `http::Request` type from the popular `http` crate
- in the function signature, the response type can be anything that implements `IntoResponse` meaning the return type can any number of things including `anyhow::Result<impl IntoResponse>` (as shown above), `impl IntoResponse`, `Response`, `anyhow::Result<Response>`, or even the `http::Response` type from the `http` crate. 
  - Note: Using the `http` crate will require you to add it to your Cargo.toml manifest (i.e., `cargo add http`).

### Making Outbound HTTP Requests

Let's see an example where the component makes an outbound HTTP request to a server, modifies the result, and then returns it:

```rust
use spin_sdk::{
    http::{IntoResponse, Request, Method, Response},
    http_component,
};

#[http_component]
async fn handle_hello_world(_req: Request) -> Result<impl IntoResponse> {
    // Create the outbound request object
    let req = Request::builder()
        .method(Method::Get)
        .uri("https://random-data-api.fermyon.app/animals/json")
        .build();

    // Send the request and await the response
    let res: Response = spin_sdk::http::send(req).await?;

    println!("{:?}", res);  // log the response
    Ok(res)
}
```

For the component above to be allowed to make the outbound HTTP request, the destination host must be declared, using the `allowed_outbound_hosts` configuration, in the Spin application's manifest (the `spin.toml` file):

```toml
spin_manifest_version = 2

[application]
name = "hello_world"
version = "0.1.0"
authors = ["Your Name <your-name@example.com>"]
description = "An example application"

[[trigger.http]]
route = "/..."
component = "hello-world"

[component.hello-world]
source = "target/wasm32-wasi/release/hello_world.wasm"
allowed_outbound_hosts = ["https://random-data-api.fermyon.app"]
[component.hello-world.build]
command = "cargo build --target wasm32-wasi --release"
watch = ["src/**/*.rs", "Cargo.toml"]
```

### Building and Running the Spin Application

Spin build can be used to build all components defined in the Spin manifest file at the same time, and also has a flag that starts the application after finishing the compilation, `spin build --up`:

```bash
$ spin build --up
Building component hello-world with `cargo build --target wasm32-wasi --release`
    Finished release [optimized] target(s) in 0.12s
Finished building all Spin components
Logging component stdio to ".spin/logs/"

Serving http://127.0.0.1:3000
Available Routes:
  hello-world: http://127.0.0.1:3000 (wildcard)
```

Once our application is running, we can make a request (by visiting `http://localhost:3000/` in a web browser) or using `curl` as shown below:

```bash
$ curl -i localhost:3000
HTTP/1.1 200 OK
content-length: 77
content-type: application/json

{"timestamp":1702599575198,"fact":"Sharks lay the biggest eggs in the world"}
```
