<div align="center">
  <h1>Project Spin</h1>
  <img src="./docs/images/spin.png" width="300"/>
  <p>Spin is a tool that allows developers to build, publish, and deploy WebAssembly workloads. It is the next version of the Fermyon runtime.</p>
</div>

## Take Spin for a spin

- [Build Spin CLI](#build-spin-cli)
- [Build and Run an HTTP application with Spin](#build-and-run-an-http-application-with-spin)
  - [Generate an HTTP Application Using a Template](#generate-an-http-application-using-a-spin-template)
  - [Build the Application](#build-the-application)
  - [Run the Application Locally](#run-the-application-locally)
- [Publish HTTP application using `cargo component`](#publish-the-application-using-cargo-component)
- [Run application with Spin from the Registry](#run-application-with-spin-from-the-registry)
- [Publishing Interfaces](#publishing-interfaces)
  - [Publish the Spin HTTP Interface](#publish-the-spin-http-interface)
  - [Use Interface in HTTP Application](#use-interface-in-http-application)

## Build Spin CLI

Clone this repository and build the Spin CLI:

```shell
$ git clone https://github.com/fermyon/spin
$ cd spin && cargo build --release
```

## Build and Run an HTTP Application with Spin

### Generate an HTTP Application Using a Spin Template

Add a new Spin template based on the `templates/spin-http` directory from
this repo:

```shell
$ spin templates add --local templates/spin-http --name spin-http
$ spin templates list
+---------------------------------------+
| Name        Repository   URL   Branch |
+=======================================+
| spin-http   local                     |
+---------------------------------------+
```

Create the application:

```shell
$ mkdir helloworld
# TODO: the name and path where the app is generated is wrong.
$ spin new --repo local --template spin-http --path .
```

### Build the Application

In the application directory:

```shell
$ cargo build --target wasm32-wasi --release
```

### Run the Application Locally

```shell
$ export RUST_LOG=spin_engine=info,spin_http,wact=info
$ spin up --local target/wasm32-wasi/release/spinhelloworld.wasm

[2021-12-07T05:09:38Z INFO  spin_engine] Execution context initialized in: 21.745807ms
[2021-12-07T05:09:49Z INFO  spin_http] Request URI: "/"
Request: Request { method: Method::Get, uri: "/", headers: [("host", "localhost:3000"), ("user-agent", "curl/7.77.0"), ("accept", "*/*")], params: [], body: Some([]) }
[2021-12-07T05:09:49Z INFO  spin_http] Response status code: 418
[2021-12-07T05:09:49Z INFO  spin_http] Request execution time: 2.773625ms
```

The application is now ready, after starting, send a request using
`curl -i localhost:3000`:

```console
$ curl -i localhost:3000
HTTP/1.1 418 I'm a teapot
content-length: 12
date: Tue, 14 Dec 2021 18:57:59 GMT

I'm a teapot
```

### Publish the application using `cargo component`

Components can be published to a bindle registry using a tool called `cargo-component`.
Download [bindle](https://github.com/deislabs/bindle) to run a bindle registry locally.
Also, download and set up [`wact` and `cargo component`](https://github.com/fermyon/wact)
for the publish functionality.

Start a bindle registry:

```shell
$ RUST_LOG=bindle=trace bindle-server --address 127.0.0.1:8080 --directory . --unauthenticated
```

Now that the application has been built, publish it to the registry.

```shell
$ cargo component publish
Published component `spinhelloworld` (version 0.1.0)
```

### Run Application with Spin from the Registry

Now that the application has been published, run it with Spin directly from the
registry:

```shell
$ spin up --bindle spinhelloworld --bindle-version 0.1.0
```

## Publishing Interfaces

In the example above, the interface (`.wit` file) was copied over to the local HTTP
application directory. You can also publish interfaces to a bindle registry for others
to consume as well as pull interfaces from a bindle registry to use. The example below
creates and publishes the spin http interface and then walks through how to consume it
in the HTTP application from the previous example.

### Publish the Spin HTTP Interface

Push the Spin HTTP interface to the registry (from the root of this repository).
This step, together with starting the registry, will not be required once we set
up a canonical registry instance:

```shell
$ wact interface publish --name fermyon/http --version 0.1.0 wit/ephemeral/spin-http.wit
```

### Use Interface in HTTP Application

1. Update `Cargo.toml` to include the following depedency, component and interface information:

```toml
[...]
[dependencies]
    # The Wact dependency generates bindings that simplify working with interfaces.
    wact = { git = "https://github.com/fermyon/wact", rev = "93a9eaeba9205918dc214a6310c0bb6e33c0e3c8" }

[workspace]

# Metadata about this component.
[package.metadata.component]
    name = "spinhelloworld"

# This component implements the fermyon/http interface.
[package.metadata.component.exports]
    fermyon-http = { name = "fermyon/http", version = "0.1.0" }
```

2. Update the application to use wact to generate and use rust bindings. In `src/lib.rs`:

```rust
// Import the HTTP objects from the generated bindings.
use fermyon_http::{Request, Response};

// Generate Rust bindings for all interfaces in Cargo.toml.
wact::component!();

struct FermyonHttp {}
impl fermyon_http::FermyonHttp for FermyonHttp {
    // Implement the `handler` entrypoint for Spin HTTP components.
    fn handler(req: Request) -> Response {
        println!("Request: {:?}", req);
        Response {
            status: 418,
            headers: None,
            body: Some("I'm a teapot".as_bytes().to_vec()),
        }
    }
}
```

3. Remove `*.wit` files from local HTTP applicaiton directory

4. In the application directory, build the component:

```shell
$ cargo build --target wasm32-wasi --release
# OR
$ cargo component build --release
```

[Run the application locally](#run-the-application-locally) to test.
