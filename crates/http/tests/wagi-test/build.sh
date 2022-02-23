#!/bin/sh
cargo build --target=wasm32-wasi --release
mv target/wasm32-wasi/release/wagi-test.wasm .
