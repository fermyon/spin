title = "Building Spin components in other languages"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/other-languages.md"
---

> This document is continuously evolving as we improve language SDKs and add
> more examples on how to build Spin components in various programming languages.

> See the document on writing [Rust](/rust-components) and [Go](/go-components)
> components for Spin for detailed guides.

WebAssembly is becoming [a popular compilation target for programming languages](https://www.fermyon.com/wasm-languages/webassembly-language-support), and as language toolchains add support for the
[WebAssembly component model](https://github.com/WebAssembly/component-model),
building Spin components will also become supported.

As a general rule:

- if your language supports the
[WebAssembly component model](https://github.com/WebAssembly/component-model),
building Spin components is supported either through an official Spin SDK
(such as [the Spin SDK for Rust](/rust-components)), or through using
bindings generators like [`wit-bindgen`](https://github.com/bytecodealliance/wit-bindgen)
(for languages such as C and C++)
- if your language compiles to WASI, but doesn't have support for the component
model, you can build [Spin HTTP components](/http-trigger) that use the
Wagi executor — for example in languages such as
[Grain](https://github.com/deislabs/hello-wagi-grain),
[AssemblyScript](https://github.com/deislabs/hello-wagi-as), or
[Python](https://github.com/fermyon/wagi-python).
- if your language doesn't currently compile to WASI, there is no way to
build and run Spin components in that programming language
