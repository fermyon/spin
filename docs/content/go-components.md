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

## Versions

TinyGo currently requires Go versions `1.15.x` through `1.17.x`. The recommendation is to use
Go version `1.17.9`, and TinyGo version `0.22.0`. Go `1.18.x` support will be added in an upcoming
TinyGo release `0.22.x`.

## HTTP components

In Spin, HTTP components are triggered by the occurrence of an HTTP request, and
must return an HTTP response at the end of their execution. Components can be
built in any language that compiles to WASI, and Go has improved support for
writing applications, through its SDK.

Building a Spin HTTP component using the Go SDK means writing a single function,
`init` — below is a complete implementation for such a component:

```go
// A Spin component written in Go that returns "Hello, Fermyon!"
package main

import (
 "fmt"
 "net/http"

 spinhttp "github.com/fermyon/spin/sdk/go/http"
)

func init() {
 spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
  w.Header().Set("Content-Type", "text/plain")
  fmt.Fprintln(w, "Hello Fermyon!")
 })
}

func main() {}
```

The important things to note in the implementation above:

- the entry point to the component is the standard `func init()` for Go programs
- handling the request is done by calling the `spinhttp.Handle` function,
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
 "bytes"
 "fmt"
 "net/http"
 "os"

 spinhttp "github.com/fermyon/spin/sdk/go/http"
)

func init() {
 spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
  r, _ := spinhttp.Get("https://some-random-api.ml/facts/dog")

  fmt.Fprintln(w, r.Body)
  fmt.Fprintln(w, r.Header.Get("content-type"))

  // `spin.toml` is not configured to allow outbound HTTP requests to this host,
  // so this request will fail.
  if _, err := spinhttp.Get("https://fermyon.com"); err != nil {
   fmt.Fprintf(os.Stderr, "Cannot send HTTP request: %v", err)
  }
 })
}

func main() {}
```

The component can be built using the `tingygo` toolchain:

```bash
$ tinygo build -wasm-abi=generic -target=wasi -no-debug -o main.wasm main.go
```

Before we can execute this component, we need to add the
`some-random-api.ml` domain to the application manifest `allowed_http_hosts`
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
allowed_http_hosts = [ "some-random-api.ml" ]
[component.trigger]
route = "/hello"
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

> You can set `allowed_http_hosts = ["insecure:allow-all"]` if you want to allow
> the component to make requests to any HTTP host. This is **NOT** recommended
> for any production or publicly-accessible application.

## Redis components

Besides the HTTP trigger, Spin has built-in support for a Redis trigger, which
will connect to a Redis instance and will execute components for new messages
on the configured channels.

> See the [Redis trigger](./redis-trigger.md) for details about the Redis trigger.

Writing a Redis component in Go also takes advantage of the SDK:

```go
package main

import (
 "fmt"

 "github.com/fermyon/spin/sdk/go/redis"
)

func init() {
 // redis.Handle() must be called in the init() function.
 redis.Handle(func(payload []byte) error {
  fmt.Println("Payload::::")
  fmt.Println(string(payload))
  return nil
 })
}

// main function must be included for the compiler but is not executed.
func main() {}
```

The manifest for a Redis application must contain the address of the Redis instance:

```toml
spin_version = "1"
name = "spin-redis"
trigger = { type = "redis", address = "redis://localhost:6379" }
version = "0.1.0"

[[component]]
id = "echo-message"
source = "main.wasm"
[component.trigger]
channel = "messages"
[component.build]
command = "tinygo build -wasm-abi=generic -target=wasi -gc=leaking -no-debug -o main.wasm main.go"
```

The application will connect to `redis://localhost:6379`, and for every new message
on the `messages` channel, the `echo-message` component will be executed:

```bash
# first, start redis-server on the default port 6379
$ redis-server --port 6379
# then, start the Spin application
$ spin build --up
INFO spin_redis_engine: Connecting to Redis server at redis://localhost:6379
INFO spin_redis_engine: Subscribed component 0 (echo-message) to channel: messages
```

For every new message on the `messages` channel:

```bash
$ redis-cli
127.0.0.1:6379> publish messages "Hello, there!"
```

Spin will instantiate and execute the component:

```bash
INFO spin_redis_engine: Received message on channel "messages"
Payload::::
Hello, there!
```

## Storing data in Redis from Go components

Using the Spin's Go SDK, you can use the Redis key/value store to publish
messages to Redis channels. This can be used from both HTTP and Redis triggered
components.

Let's see how we can use the Go SDK to connect to Redis:

```go
package main

import (
 "net/http"
 "os"

 spin_http "github.com/fermyon/spin/sdk/go/http"
 "github.com/fermyon/spin/sdk/go/redis"
)

func init() {
 // handler for the http trigger
 spin_http.Handle(func(w http.ResponseWriter, r *http.Request) {

  // addr is the environment variable set in `spin.toml` that points to the
  // address of the Redis server.
  addr := os.Getenv("REDIS_ADDRESS")

  // channel is the environment variable set in `spin.toml` that specifies
  // the Redis channel that the component will publish to.
  channel := os.Getenv("REDIS_CHANNEL")

  // payload is the data publish to the redis channel.
  payload := []byte(`Hello redis from tinygo!`)

  if err := redis.Publish(addr, channel, payload); err != nil {
   http.Error(w, err.Error(), http.StatusInternalServerError)
   return
  }

  // set redis `mykey` = `myvalue`
  if err := redis.Set(addr, "mykey", []byte("myvalue")); err != nil {
   http.Error(w, err.Error(), http.StatusInternalServerError)
   return
  }

  // get redis payload for `mykey`
  if payload, err := redis.Get(addr, "mykey"); err != nil {
   http.Error(w, err.Error(), http.StatusInternalServerError)
  } else {
   w.Write([]byte("mykey value was: "))
   w.Write(payload)
  }
 })
}

func main() {}
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
> in the [Spin repository on GitHub](https://github.com/fermyon/spin/tree/main/examples/tinygo-outbound-redis).

## Using Go packages in Spin components

Any
[package from the Go standard library](https://tinygo.org/docs/reference/lang-support/stdlib/) that can be imported in TinyGo and that compiles to
WASI can be used when implementing a Spin component.

> Make sure to read [the page describing the HTTP trigger](./http-trigger.md) for more
> details about building HTTP applications.
