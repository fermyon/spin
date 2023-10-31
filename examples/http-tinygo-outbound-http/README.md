# Making outbound HTTP requests from TinyGo Spin components

The TinyGo SDK for building Spin components allows us to granularly allow
components to send HTTP requests to certain hosts. This is configured in
`spin.toml`.

> For more information and examples for using TinyGo with WebAssembly, check
> [the official TinyGo documentation](https://tinygo.org/docs/guides/webassembly/)
> and
> [the Wasm examples](https://github.com/tinygo-org/tinygo/tree/release/src/examples/wasm).

Creating and sending HTTP requests from Spin components closely follows the Go
`net/http` API.  See [tinygo-hello/main.go](./tinygo-hello/main.go).

Building this as a WebAssembly module can be done using the `tinygo` compiler:

```shell
$ spin build
Building component outbound-http-to-same-app with `tinygo build -target=wasi -gc=leaking -no-debug -o main.wasm main.go`
Working directory: "./outbound-http-to-same-app"
Building component tinygo-hello with `tinygo build -target=wasi -gc=leaking -no-debug -o main.wasm main.go`
Working directory: "./tinygo-hello"
Finished building all Spin components
```

The component configuration must contain a list of all hosts allowed to send
HTTP requests to, otherwise sending the request results in an error:

```
Cannot send HTTP request: Destination not allowed: <URL>
```

The `tinygo-hello` component has the following allowed hosts set:

```toml
[component.tinygo-hello]
source = "tinygo-hello/main.wasm"
allowed_outbound_hosts = [
    "https://random-data-api.fermyon.app",
    "https://postman-echo.com",
]
```

And the `outbound-http-to-same-app` uses the dedicated `self` keyword to enable making
a request to another component in this same app, via a relative path (in this case, the component
is `tinygo-hello` at `/hello`):

```toml
[component.outbound-http-to-same-app]
source = "outbound-http-to-same-app/main.wasm"
# Use self to make outbound requests to components in the same Spin application.
allowed_outbound_hosts = ["http://self"]
```

At this point, we can execute the application with the `spin` CLI:

```shell
$ RUST_LOG=spin=trace,wasi_outbound_http=trace spin up
```

The application can now receive requests on `http://localhost:3000/hello`:

```shell
$ curl -i localhost:3000/hello -X POST -d "hello there"
HTTP/1.1 200 OK
content-length: 976
date: Thu, 26 Oct 2023 18:26:17 GMT

{{"timestamp":1698344776965,"fact":"Reindeer grow new antlers every year"}}
...
```

As well as via the `/outbound-http-to-same-app` path to verify outbound http to the `tinygo-hello` component:

```shell
$ curl -i localhost:3000/outbound-http-to-same-app
HTTP/1.1 200 OK
content-length: 946
date: Thu, 26 Oct 2023 18:26:53 GMT

{{{"timestamp":1698344813408,"fact":"Some hummingbirds weigh less than a penny"}}
...
```

## Notes

- this only implements sending HTTP/1.1 requests
- requests are currently blocking and synchronous
