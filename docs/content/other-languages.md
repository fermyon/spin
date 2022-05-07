title = "Building Spin components in other languages"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/other-languages.md"
---

> This document is continuously evolving as we improve language SDKs and add
> more examples on how to build Spin components in various programming languages.

> See the document on writing [Rust](./rust-components.md) and [Go](./go-components.md)
> components for Spin for detailed guides.

WebAssembly is becoming [a popular compilation target for programming languages](https://www.fermyon.com/wasm-languages/webassembly-language-support), and as language toolchains add support for the
[WebAssembly component model](https://github.com/WebAssembly/component-model),
building Spin components will also become supported.

As a general rule:

- if your language supports the
[WebAssembly component model](https://github.com/WebAssembly/component-model),
building Spin components is supported either through an official Spin SDK
(such as [the Spin SDK for Rust](./rust-components.md)), or through using
bindings generators like [`wit-bindgen`](https://github.com/bytecodealliance/wit-bindgen)
(for languages such as C and C++)
- if your language compiles to WASI, but doesn't have support for the component
model, you can build [Spin HTTP components](./http-trigger.md) that use the
Wagi executor — for example in languages such as
[Grain](https://github.com/deislabs/hello-wagi-grain),
[AssemblyScript](https://github.com/deislabs/hello-wagi-as), or
[Python](https://github.com/fermyon/wagi-python).
- if your language doesn't currently compile to WASI, there is no way to
build and run Spin components in that programming language

> Make sure to check out [a more complex Spin application with components built
in multiple programming languages](https://github.com/fermyon/spin-kitchensink/).

## AssemblyScript

[AssemblyScript](https://www.assemblyscript.org/) is a TypeScript-based language that compiles directly to WebAssembly.
AssemblyScript has WASI/Wagi support, and so can be used with Spin.

- The [AssemblyScript entry in the Wasm Language Guide](https://www.fermyon.com/wasm-languages/assemblyscript) includes a full example
- The [Spin Kitchen Sink](https://github.com/fermyon/spin-kitchensink) repo has an AssemblyScript demo
- An [example AssemblyScript app](https://github.com/deislabs/hello-wagi-as) designed for Wagi runs on Spin

## C/C++

C and C++ are both broadly supported in the WebAssembly ecosystem. WASI/Wagi support means that both can be used to write Spin apps.

- The [C entry in the Wasm Language Guide](https://www.fermyon.com/wasm-languages/c-lang) has examples.
- The [C++ entry in the Wasm Language Guide](https://www.fermyon.com/wasm-languages/cpp) has specific caveats for writing C++ (like exception handling)
- The [yo-wasm](https://github.com/deislabs/yo-wasm) project makes setting up C easier.

## C# and .NET languages

.NET has experimental support for WASI, so many (if not all) .NET languages, including C# and F#, can be used to write Spin applications.

- The [C# entry in the Wasm Language Guide](https://www.fermyon.com/wasm-languages/c-sharp) has a full example.
- The [Spin Kitchen Sink repo](https://github.com/fermyon/spin-kitchensink) has two C# examples and one F# example.

## Grain

[Grain](https://grain-lang.org/), a new functional programming language, has WASI/Wagi support and can be used to write Spin apps.

- The [Grain entry in the Wasm Language Guide](https://www.fermyon.com/wasm-languages/grain) has details
- A simple [Hello World example](https://github.com/deislabs/hello-wagi-grain) shows how to use Grain
- For a production-quality example. the [Wagi Fileserver](https://github.com/deislabs/wagi-fileserver) is written in Grain

## Python

Python's interpreter can be compiled to WebAssembly, and it has WASI support. It is known to work for Spin.

- The [Spin Kitchen Sink](https://github.com/fermyon/spin-kitchensink) repo includes a Python example
- The [Python entry in the Wasm Language Guide](https://www.fermyon.com/wasm-languages/python) lists two implementations
- There is a Fermyon blog post about [using Python with WAGI](https://www.fermyon.com/blog/python-wagi)
- The [Python docs](https://pythondev.readthedocs.io/wasm.html) have a page on WebAssembly
- SingleStore also has [a Python build](https://github.com/singlestore-labs/python-wasi) that uses mainline Python

## Ruby

Upstream [Ruby](https://www.ruby-lang.org/en/) officially supports WebAssembly and WASI, and we here at Fermyon have successfully run Ruby apps in Spin.

- The [Ruby entry in the Wasm Language Guide](https://www.fermyon.com/wasm-languages/ruby) has the latest information
- [Ruby's 3.2.0 Preview 1 release notes](https://www.ruby-lang.org/en/news/2022/04/03/ruby-3-2-0-preview1-released/) detail WASI support

## Zig

Zig is a low-level systems language that has support for Wasm and WASI, and can be used to write Spin apps.

- The [Zig entry in the Wasm Language Guide](https://www.fermyon.com/wasm-languages/zig) covers the basics
- Zig's [0.4 release notes](https://ziglang.org/download/0.4.0/release-notes.html#WebAssembly-Support) explain WebAssembly support