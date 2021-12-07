# Project Spin

Project Spin is the next version of the Fermyon runtime.

### Building and running a Spin HTTP application

First, you need the following tools:

- [Wact](https://github.com/fermyon/wact)
- [Bindle server](https://github.com/deislabs/bindle)

Then, clone this repository and build the Spin CLI:

```shell
$ git clone https://github.com/fermyon/spin
$ cd spin && cargo build --release
```

- start a WebAssembly registry instance (Bindle):

```shell
$ RUST_LOG=bindle=trace bindle-server --address 127.0.0.1:8080 --directory . --unauthenticated
```

- push the Spin HTTP interface to the registry (from the root of this
  repository). This step, together with starting the registry, will not be
  required once we set up a canonical registry instance:

```shell
$ wact interface push --name fermyon/http --version 0.1.0 crates/http/spin_http_v01.wai
pushed interface `fermyon/http` (version 0.1.0)
```

- now we should be ready to start writing a new application. First, add a new
  Spin template based on the `templates/spin-http` directory from this repo:

```shell
$ spin templates add --local templates/spin-http --name spin-http
$ spin templates list
+---------------------------------------+
| Name        Repository   URL   Branch |
+=======================================+
| spin-http   local                     |
+---------------------------------------+
```

- now we can create the application:

```shell
$ mkdir helloworld
# TODO: the name and path where the app is generated is wrong.
$ spin templates generate --repo local --template spin-http --path .
```

- in the application directory, pull the interfaces, then build:

```shell
$ wact cargo pull
$ cargo build --target wasm32-wasi --release
```

- the application is now ready, after starting, send a request using
  `curl -i localhost:3000`:

```shell
$ export RUST_LOG=spin_engine=info,spin_http,wact=info
$ spin up --local target/wasm32-wasi/release/spinhelloworld.wasm

[2021-12-07T05:09:38Z INFO  spin_engine] Execution context initialized in: 21.745807ms
[2021-12-07T05:09:49Z INFO  spin_http] Request URI: "/"
Request: Request { method: Method::Get, uri: "/", headers: [("host", "localhost:3000"), ("user-agent", "curl/7.77.0"), ("accept", "*/*")], params: [], body: Some([]) }
[2021-12-07T05:09:49Z INFO  spin_http] Response status code: 418
[2021-12-07T05:09:49Z INFO  spin_http] Request execution time: 2.773625ms
```
