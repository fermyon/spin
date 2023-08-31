## longevity-apps-test

longevity test is to ensure `wasm` file(s) compiled with a version of `Spin` continues to work with runtime of future version of `Spin`. 

The current wasm files are created using following templates with `Spin v0.9.0`

- http-go
- http-rust
- http-js
- http-ts

The `wasm` files are built using `spin build` and copied over here for validation.

## How to re-generate the wasm files

> When updating the wasm files, we also need to update the value of const `SPIN_VERSION_USED_TO_BUILD_APP` in `tests/testcases/mod.rs::longevity_apps_works`. It will enable us to efficiently track the change that triggered the backward incompatibility issue.

### Install plugin and templates

```
spin plugin update
spin plugin install js2wasm --yes
spin templates install --git https://github.com/fermyon/spin
spin templates install --git https://github.com/fermyon/spin-js-sdk
```

### Create app using template and generate wasm modules

```
spin new http-go http-go-test
cd http-go-test
spin build
cp main.wasm longevity-go.wasm
```

```
spin new http-rust http-rust-test
cd http-rust-test
spin build
cp target/wasm32-wasi/release/http_rust_test.wasm longevity-rust.wasm
```

```
spin new http-js http-js-test
cd http-js-test
npm install
spin build
cp target/spin-http-js.wasm longevity-javascript.wasm
```

```
spin new http-ts http-ts-test
cd http-ts-test
npm install
spin build
cp target/spin-http-js.wasm longevity-typescript.wasm
```

