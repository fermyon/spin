title = "Building Spin components in Go"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/go-components.md"
---

[TinyGo](https://tinygo.org/) is an implementation of the
[Go programming language](https://go.dev/) for embedded systems and WebAssembly.
The Spin SDK for Go uses
[TinyGo's WASI support](https://tinygo.org/docs/reference/usage/important-options/)
to build programs written in Go as Spin components.

> This guide assumes you are familiar with the Go programming language, and that
> you have
> [configured the TinyGo toolchain locally](https://tinygo.org/getting-started/install/).
Using TinyGo to compile components for Spin is currently required, as the
[Go compiler doesn't currently have support for compiling to WASI](https://github.com/golang/go/issues/31105).

> All examples from this page can be found in [the Spin repository on GitHub](https://github.com/fermyon/spin/tree/main/examples).

## HTTP components

In Spin, HTTP components are triggered by the occurrence of an HTTP request, and
must return an HTTP response at the end of their execution. Components can be
built in any language that compiles to WASI, and Go has improved support for
writing applications, through its SDK.

Building a Spin HTTP component using the Go SDK means writing a single function,
`main` — below is a complete implementation for such a component:

```go
// A Spin component written in Go that returns "Hello, Fermyon!"
package main

import (
 "fmt"
 "net/http"

 spin "github.com/fermyon/spin/sdk/go/http"
)

func main() {
 spin.HandleRequest(func(w http.ResponseWriter, r *http.Request) {
  fmt.Fprintln(w, "Hello, Fermyon!")
 })
}
```

The important things to note in the implementation above:

- the entry point to the component is the standard `func main()` for Go programs
- handling the request is done by calling the `spin.HandleRequest` function,
which takes a `func(w http.ResponseWriter, r *http.Request)` as parameter — these
contain the HTTP request and response writer you can use to handle the request
- the HTTP objects (`*http.Request`, `http.Response`, and `http.ResponseWriter`)
are the Go objects from the standard library, so working with them should feel
familiar if you are a Go developer

## Sending outbound HTTP requests

If allowed, Spin components can send outbound requests to HTTP endpoints. Let's
see an example of a component that makes a request to
[an API that returns random dog facts](https://some-random-api.ml/facts/dog) and
inserts a custom header into the response before returning:

```go
// A Spin component written in Go that sends a request to an API
// with random dog facts.
package main

import (
 "fmt"
 "net/http"
 "os"

 spin_http "github.com/fermyon/spin/sdk/go/http"
)

func main() {
 spin_http.HandleRequest(func(w http.ResponseWriter, r *http.Request) {
  res, err := spin_http.Get("https://some-random-api.ml/facts/dog")
  if err != nil {
   fmt.Fprintf(os.Stderr, "Cannot send HTTP request: %v", err)
  }
  // optionally writing a response header
  w.Header().Add("server", "spin/0.1.0")
  fmt.Fprintln(w, res.Body)
 })
}
```

The component can be built using the `tingygo` toolchain:

```bash
$ tinygo build -wasm-abi=generic -target=wasi -o main.wasm main.go
```

Before we can execute this component, we need to add the
`https://some-random-api.ml` domain to the application manifest `allowed_http_hosts`
list containing the list of domains the component is allowed to make HTTP
requests to:

```toml
# spin.toml
spin_version = "1"
name = "spin-hello-tinygo"
trigger = { type = "http", base = "/" }
version = "1.0.0"

[[component]]
id = "tinygo-hello"
source = "main.wasm"
allowed_http_hosts = [ "https://some-random-api.ml" ]
[component.trigger]
route = "/hello"
executor = { type = "wagi" }
```

> Spin HTTP components written in Go must currently use the Wagi executor.

Running the application using `spin up --file spin.toml` will start the HTTP
listener locally (by default on `localhost:3000`), and our component can
now receive requests in route `/hello`:

```bash
$ curl -i localhost:3000/hello
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
server: spin/0.1.0
content-length: 85
date: Fri, 18 Mar 2022 23:27:33 GMT

{{"fact":"Seventy percent of people sign their dog's name on their holiday cards."}}
```

> Without the `allowed_http_hosts` field populated properly in `spin.toml`,
> the component would not be allowed to send HTTP requests, and sending the
> request would generate in a "Destination not allowed" error.

## Using Go packages in Spin components

Any
[package from the Go standard library](https://tinygo.org/docs/reference/lang-support/stdlib/) that can be imported in TinyGo and that compiles to
WASI can be used when implementing a Spin component.

> Make sure to read [the page describing the HTTP trigger](/http-trigger) for more
> details about building HTTP applications.
