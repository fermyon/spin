# Building HTTP applications using Spin

Currently, the only applications that can be built with Spin are web based, or
applications that are invoked as the result of an HTTP request, and which return
an HTTP response. This is because HTTP workloads appear to be the most important
for event-driven Functions-as-a-Service workloads, and we think initially serve
the most popular use cases.

> The extensible nature of Spin allows anyone to extend it by building more
> triggers (see the [architecture](./architecture.md) and
> [contributing](./contributing.md) documents), and we are experimenting with a
> new trigger that invokes components for new payloads on a Redis message queue
> (see [#59](https://github.com/fermyon/spin/issues/59)).

Spin is built on top of the
[WebAssembly component model](https://github.com/WebAssembly/component-model).
We _strongly_ believe it represents the future of WebAssembly, and that it will
enable scenarios that are simply not possible today (for example dynamic linking
and transitive dependencies). As a result, the Spin HTTP trigger (and executor)
is defined using [WebAssembly interfaces](../wit/ephemeral), and the
[SDK for building Rust components](../sdk/rust) is built on top of the Rust
implementation and bindings generator for WebAssembly components.

But the WebAssembly component model is currently in its early stages. This means
only a few languages fully implement it. While language communities implement
the component model, we want to allow developers to use
[any language that compiles to WASI](https://www.fermyon.com/wasm-languages/webassembly-language-support)
to build Spin HTTP applications. This is why we currently implement a Wagi
executor which supports [Wagi](https://github.com/deislabs/wagi)-based
components that expect the HTTP request using the module's standard input, and
return the HTTP response using the module's standard output, following
[the CGI specification](https://tools.ietf.org/html/rfc3875). As a programming
language adds support for the component model, we plan to enable better support
for it in Spin, and eventually only support Spin applications that implement the
WebAssembly component model.

## Building HTTP components in Rust

We believe the Rust SDK offers the best experience for building Spin HTTP
components, and this is the recommended way of writing Spin components in Rust.

Building such a component in Rust requires writing a function that takes an HTTP
`Request` and returns an HTTP `Response`, annotated with a special Spin
procedural macro. Below is a complete component implementation:

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

The important things to note in the function above:

- the `spin_sdk::http_component` macro — this marks the function as the
  entrypoint for the Spin component
- the function signature — `fn hello_world(req: Request) -> Result<Response>` —
  the Spin HTTP component uses the HTTP objects from the popular Rust crate
  [`http`](https://crates.io/crates/http), and the request and response bodies
  are optionally using [`bytes::Bytes`](https://crates.io/crates/bytes)

### Making outbound HTTP requests

This SDK includes the ability to send outbound HTTP requests using the
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
allowedHttpHosts = [ "https://fermyon.com" ]
[component.trigger]
route = "/hello"
```

Making a request to this component, we can see the appended header, and that the
response contains the expected body:

```shell
$ curl -I localhost:3000/hello
HTTP/1.1 200 OK
content-length: 29350
content-type: text/html; charset=utf-8
server: spin/0.1.0 # the header added by our component
```

Any Rust crate that compiles to `wasm32-wasi` can be used as dependency in Rust
components.

As the Spin framework evolves, the Spin SDK will continue adding functionality
that improves the experience for building Spin components (such as implementing
interfaces for popular functionality such as
[object storage](https://github.com/fermyon/spin/issues/48),
[key/value stores](https://github.com/fermyon/spin/issues/47), or
[neural networks](https://github.com/fermyon/spin/issues/50)).

As more languages support the WebAssembly component model, our goal is to
develop language SDKs for such popular languages.

## Building HTTP components using the Wagi executor

You can use any language that compiles to WASI to build an HTTP component using
the [Wagi](https://github.com/deislabs/wagi) executor.

Wagi is a project that lets you write HTTP handlers using nothing but a
language's standard library, following
[the CGI specification](https://tools.ietf.org/html/rfc3875).

For example, here is a complete Wagi component written in Swift:

```swift
print("content-type: text/html; charset=UTF-8\n\n");
print("hello world\n");
```

Here is another example, this time written in [Grain](https://grain-lang.org/),
a new programming language that natively targets WebAssembly:

```js
import Process from "sys/process";
import Array from "array";

print("content-type: text/plain\n");

// This will print all the Wagi env variable
print("==== Environment: ====");
Array.forEach(print, Process.env());

// This will print the route path followed by each query
// param. So /foo?bar=baz will be ["/foo", "bar=baz"].
print("==== Args: ====");
Array.forEach(print, Process.argv());
```

> You can find examples on how to build Wagi applications in
> [the DeisLabs GitHub organization](https://github.com/deislabs?q=wagi&type=public&language=&sort=).

In short, read HTTP headers from environment variables and the HTTP body from
standard input, and return the response to standard output. You can follow the
[Wagi guide](https://github.com/deislabs/wagi/blob/main/docs/writing_modules.md)
on writing modules (note that a module declaring its subroutes will not be
implemented in Spin).

## Writing HTTP components in (Tiny)Go

Below is a complete implementation for a Spin HTTP component in Go:

```go
package main

import (
 "io"
 "net/http"

 spin_http "github.com/fermyon/spin-sdk"
)

func main() {
 spin_http.HandleRequest(func(w http.ResponseWriter, r *http.Request) {
  io.WriteString(w, "Hello, Fermyon!")
 })
}
```

## The default headers set in Spin HTTP components

Spin sets a few default headers on the request based on the base path, component
route, and request URI, which will always be available when writing a module:

- `X_FULL_URL` - the full URL of the request —
  `http://localhost:3000/test/wagi/abc/def?foo=bar`
- `PATH_INFO` - the path info, relative to both the base application path _and_
  component route — in our example, where the base path is `/test`, and the
  component route is `/hello`, this is `/abc/def`.
- `X_MATCHED_ROUTE` - the base path and route pattern matched (including the
  wildcard pattern, if applicable) (this updates the header set in Wagi to
  include the base path) — in our case `"/test/hello/..."`.
- `X_RAW_COMPONENT_ROUTE` - the route pattern matched (including the wildcard
  pattern, if applicable) — in our case `/hello/...`.
- `X_COMPONENT_ROUTE` - the route path matched (stripped of the wildcard
  pattern) — in our case `/hello`
- `X_BASE_PATH` - the application base path — in our case `/test`.

Besides the headers above, components that use the Wagi executor also have
available
[all headers set by Wagi, following the CGI spec](https://github.com/deislabs/wagi/blob/main/docs/environment_variables.md).
