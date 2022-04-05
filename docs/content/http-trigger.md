title = "The Spin HTTP trigger"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/http-trigger.md"
---

An important workload in event-driven environments is represented by HTTP
applications, and Spin has built-in support for creating and running HTTP
components. This document presents an overview of the HTTP trigger, as well as
some implementation details around the WebAssembly component model and how it
is used in Spin.

The HTTP trigger in Spin is a web server. It listens for incoming requests and
based on the [application manifest](/configuration), it routes them to an
_executor_ which instantiates the appropriate component, executes its
entry point function, then returns an HTTP response.

Creating an HTTP application is done when [configuring the application](/configuration)
by defining the top-level application trigger:

```toml
# spin.toml
trigger = { type = "http", base = "/" }
```

Then, when defining the component (in `spin.toml`), there are two pieces of
configuration that can be set for the component trigger: the route,
and the _HTTP executor_ (see details below about executors). For example:

- an HTTP component configured on the `/hello` route that uses the Spin executor:

```toml
[component.trigger]
route = "/hello"
executor = { type = "spin" }
```

- an HTTP component configured on the `/goodbye` route that uses the Wagi executor:

```toml
[component.trigger]
route = "/goodbye"
executor = { type = "wagi" }
```

## Routing

Routing an incoming request to a particular component is done using the
application base path (`base` in `spin.toml`) and the component defined routes
(`route` in the component configuration) by prefixing the application base path
to all component routes defined for that application.

For example, if the application `base` path is `base = /base`, and a component
has defined `route = /foo`, that component will be executed for requests on
`http(s)://<spin-up-defined-address-and-port>/base/foo`.

Components can either define exact routes, for example `route = /bar/baz`, where
the component will be invoked only for requests on `/base/bar/baz`, or they
can define a wildcard as the last path segment, for example `route = /bar/baz/...`,
which means the component will be invoked for every request starting with the
`/base/bar/baz/` prefix (such as `/base/bar/baz`, `/base/bar/baz/qux`,
`/base/bar/baz/qux/quux` and so on).

If multiple components could potentially handle the same request based on their
defined routes, the last component defined in `spin.toml` takes precedence.
In the following example:

```toml
# spin.toml

trigger = { type = "http", base = "/"}

[[component]]
id = "component-1"
[component.trigger]
route = "/..."

[[component]]
id = "component-2"
[component.trigger]
route = "/foo/..."
```

Any request starting with the  `/foo/` prefix  will be handled by `component-2`,
which is the last one defined in `spin.toml`.

Every HTTP application has a special route always configured at `/healthz`, which
returns `OK 200` when the Spin instance is healthy.

Once Spin selects a component to handle an incoming request based on the route
configuration, it will instantiate and execute that component based on its
defined _HTTP executor_, and the next sections explore the two ways of building
HTTP components based on the two available executors.

## The Spin HTTP executor

Spin is built on top of the
[WebAssembly component model](https://github.com/WebAssembly/component-model).
We _strongly_ believe the component model represents the future of WebAssembly,
and we are working with the [Bytecode Alliance](https://bytecodealliance.org)
community on building exciting new features and tools for it. As a result, the
Spin HTTP _executor_ is defined using WebAssembly interfaces.

> The WebAssembly component model is in its early stages, and during the `0.x`
> releases of Spin, the triggers and application entry points will suffer
> breaking changes, particularly around the primitive types used to define
> the HTTP objects and function signatures — i.e. bodies will become streams,
> handler functions will become asynchronous.

We define the HTTP objects as
[WebAssembly Interface (WIT)](https://github.com/bytecodealliance/wit-bindgen/blob/main/WIT.md)
objects, currently using _records_:

```fsharp
// wit/ephemeral/http-types.wit

// The HTTP status code.
type http-status = u16
// The HTTP body.
type body = list<u8>
// The HTTP headers represented as a list of (name, value) pairs.
type headers = list<tuple<string, string>>
// The HTTP parameter queries, represented as a list of (name, value) pairs.
type params = list<tuple<string, string>>
// The HTTP URI of the current request.
type uri = string
// The HTTP method.
enum method { get, post, put,... }

// An HTTP request.
record request {
    method: method,
    uri: uri,
    headers: headers,
    params: params,
    body: option<body>,
}

// An HTTP response.
record response {
    status: http-status,
    headers: option<headers>,
    body: option<body>,
}
```

> The same HTTP types are also used to model the API for sending outbound
> HTTP requests, and you can see its implementation in
> [the WASI toolkit repository](https://github.com/fermyon/wasi-experimental-toolkit).

Then, we define the entry point for a Spin HTTP component:

```fsharp
// wit/ephemeral/spin-http.wit

use * from http-types
// The entry point for an HTTP handler.
handle-http-request: function(req: request) -> response
```

This is the function signature that all HTTP components must implement, and
which is used by the Spin HTTP executor when instantiating and invoking the
component.
This interface (`spin-http.wit`) can be directly used together with the
[Bytecode Alliance `wit-bindgen` project](https://github.com/bytecodealliance/wit-bindgen)
to build a component that the Spin HTTP executor can invoke.
This is exactly how [the Rust SDK for Spin](/rust-components) is built, and,
as more languages add support for the component model, how we plan to add
support for them as well.

## The Wagi HTTP executor

The WebAssembly component model proposal is currently in its early stages, which
means only a few programming languages fully implement it. While the language
communities implement toolchain support for the component model (for emitting
components and for automatically generating bindings for importing other
components), we want to allow developers to use any language that compiles to
WASI to build Spin HTTP applications. This is why Spin currently implements an
HTTP executor based on [Wagi](https://github.com/deislabs/wagi), or the
WebAssembly Gateway Interface, a project that implements the
[Common Gateway Interface](https://datatracker.ietf.org/doc/html/rfc3875)
specification for WebAssembly.

> Spin will keep supporting the Wagi-based executor while language toolchains
> add support for the WebAssembly component model. When enough programming
> languages have implemented the component model, we will work with the Spin
> community to decide when to deprecate the Wagi executor.

Wagi allows a module built in any programming language that compiles to [WASI](https://wasi.dev/)
to handle an HTTP request by passing the HTTP request information to the module's
standard input, environment variables, and arguments, and expecting the HTTP
responses through the module's standard output.
This means that if a language has support for the WebAssembly System Interface,
it can be used to build Spin HTTP components.
The Wagi model is only used to parse the HTTP request and response. Everything
else — defining the application, running it, or [distributing](/distributing-apps)
is done the same way as a component that uses the Spin executor.

Building a Wagi component in a particular programming language that can compile
to `wasm32-wasi` does not require any special libraries — instead,
[building Wagi components](https://github.com/deislabs/wagi/tree/main/docs) can
be done by reading the HTTP request from the standard input and environment
variables, and sending the HTTP response to the module's standard output.

In pseudo-code, this is the minimum required in a Wagi component:

- either the `content-media` or `location` headers must be set — this is done by
printing its value to standard output
- an empty line between the headers and the body
- the response body printed to standard output

```
print("content-type: text/html; charset=UTF-8\n\n");
print("hello world\n");
```

The [Go SDK for Spin](/go-components) is built on the Wagi executor support.
Here is another example, written in [Grain](https://grain-lang.org/),
a new programming language that natively targets WebAssembly:

```js
import Process from "sys/process";
import Array from "array";

print("content-type: text/plain\n");

// This will print all the Wagi env variables
print("==== Environment: ====");
Array.forEach(print, Process.env());

// This will print the route path followed by each query
// param. So /foo?bar=baz will be ["/foo", "bar=baz"].
print("==== Args: ====");
Array.forEach(print, Process.argv());
```

> You can find examples on how to build Wagi applications in
> [the DeisLabs GitHub organization](https://github.com/deislabs?q=wagi&type=public&language=&sort=).

### The default headers set in Spin HTTP components

Spin sets a few default headers on the request based on the base path, component
route, and request URI, which will always be available when writing a module:

- `spin-full-url` - the full URL of the request —
  `http://localhost:3000/test/wagi/abc/def?foo=bar`
- `spin-path-info` - the path info, relative to both the base application path _and_
  component route — in our example, where the base path is `/test`, and the
  component route is `/hello`, this is `/abc/def`.
- `spin-matched-route` - the base path and route pattern matched (including the
  wildcard pattern, if applicable) (this updates the header set in Wagi to
  include the base path) — in our case `"/test/hello/..."`.
- `spin-raw-component-route` - the route pattern matched (including the wildcard
  pattern, if applicable) — in our case `/hello/...`.
- `spin-component-route` - the route path matched (stripped of the wildcard
  pattern) — in our case `/hello`
- `spin-base-path` - the application base path — in our case `/test`.

### The default headers set in Wagi HTTP components

For Wagi HTTP components, the following are set as environment variables for the
handler WebAssembly modules:

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

Besides the headers above, components that use the Wagi executor also have set
[all headers set by Wagi, following the CGI spec](https://github.com/deislabs/wagi/blob/main/docs/environment_variables.md).
