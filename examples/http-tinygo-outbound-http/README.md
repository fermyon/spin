# Making outbound HTTP requests from TinyGo Spin components

The TinyGo SDK for building Spin components allows us to granularly allow
components to send HTTP requests to certain hosts. This is configured in
`spin.toml`.

> For more information and examples for using TinyGo with WebAssembly, check
> [the official TinyGo documentation](https://tinygo.org/docs/guides/webassembly/)
> and
> [the Wasm examples](https://github.com/tinygo-org/tinygo/tree/release/src/examples/wasm).

Creating and sending HTTP requests from Spin components closely follows the Go
`net/http` API:

```go
 r1, err := spin_http.Get("https://some-random-api.ml/facts/dog")
 r2, err := spin_http.Post("https://postman-echo.com/post", "text/plain", bytes.NewBufferString("Hello there!"))

req, err := http.NewRequest("PUT", "https://postman-echo.com/put", bytes NewBufferString("General Kenobi!"))
req.Header.Add("foo", "bar")
r3, err := spin_http.Send(req)
```

Building this as a WebAssembly module can be done using the `tinygo` compiler:

```shell
$ go mod tidy
$ spin build
Executing the build command for component tinygo-hello: tinygo build -target=wasi -gc=leaking -no-debug -o main.wasm main.go
Successfully ran the build command for the Spin components.
```

The component configuration must contain a list of all hosts allowed to send
HTTP requests to, otherwise sending the request results in an error:

```
Cannot send HTTP request: Destination not allowed: <URL>
```

```toml
[[component]]
id = "tinygo-hello"
source = "main.wasm"
allowed_http_hosts = [ "https://some-random-api.ml", "https://postman-echo.com" ]
[component.trigger]
route = "/hello"
[component.build]
command = "tinygo build -target=wasi -gc=leaking -no-debug -o main.wasm main.go"
```

At this point, we can execute the application with the `spin` CLI:

```shell
$ RUST_LOG=spin=trace,wasi_outbound_http=trace spin up
```

The application can now receive requests on `http://localhost:3000/hello`:

```shell
$ curl -i localhost:3000/hello -X POST -d "hello there"

HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 789
date: Wed, 09 Mar 2022 17:23:08 GMT
...
```

## Notes

- this only implements sending HTTP/1.1 requests
- requests are currently blocking and synchronous
