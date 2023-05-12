# Spin Wagi HTTP example

This example shows how to run a Spin application serving routes from two programs written in different languages (Rust and C++) using both the Spin executor and the Wagi executor.

## Compile Applications to Wasm

```
cd http-rust
cargo build --release
```

```
cd ../wagi-http-cpp
make build
```

## Spin up

From application root:

```
RUST_LOG=spin=trace spin up -f spin.toml
```

Curl the hello route:

```
$ curl -i localhost:3000/hello
HTTP/1.1 200 OK
content-type: application/text
content-length: 7
date: Thu, 10 Mar 2022 21:38:34 GMT

Hello
```

Curl the goodbye route:

```
$ curl -i localhost:3000/goodbye
HTTP/1.1 200 OK
foo: bar
content-length: 7
date: Thu, 10 Mar 2022 21:38:58 GMT

Goodbye
```
