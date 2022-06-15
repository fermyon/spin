## Spin HTTP Example in C++

This is a simple example of a [Spin HTTP
trigger](https://spin.fermyon.dev/http-trigger) implemented in C++.

### Building and Running

First install [Rust](https://rustup.rs) and [Spin](https://github.com/fermyon/spin).

Next, grab the latest [WASI SDK](https://github.com/WebAssembly/wasi-sdk)
release and place it at /opt/wasi-sdk on your filesystem.  For Linux, this would
look something like:

```bash
curl -OL https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-14/wasi-sdk-14.0-linux.tar.gz
tar xf wasi-sdk-14.0-linux.tar.gz
sudo mv wasi-sdk-14.0 /opt/wasi-sdk
```

Then install a compatible version of
[wit-bindgen](https://github.com/bytecodealliance/wit-bindgen).  As of this
writing, Spin uses a specific revision, which you can install like so:

```bash
cargo install --git https://github.com/bytecodealliance/wit-bindgen --rev dde4694aaa6acf9370206527a798ac4ba6a8c5b8 wit-bindgen-cli
```

Finally, build and run the example from within this directory:

```bash
spin build
spin up
```

You can test the trigger using e.g.:

```bash
curl -v http://127.0.0.1:3000/hello
```
