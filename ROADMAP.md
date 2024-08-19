# Spin Roadmap

Spin is an open-source framework designed to facilitate the development and deployment of event-driven, serverless applications using WebAssembly (Wasm). Spin provides a lightweight runtime environment that encompasses Wasmtime for running fast and secure applications integrating the latest standards in the WebAssembly community. We’re excited for the growing engagement in the Spin community and for the hard earned continued development and progress in the WebAssembly community that are incorporated in the Spin project.

Taking these considerations into account and feedback from the community, the Spin project is focusing on features and improvements both as a developer tool and as an extensible runtime environment. The following is a non-exhaustive list:

1. Composition and polyglot development experience
   
   WebAssembly components unlock a new opportunity as it relates to the polyglot development experience. In Spin, we seek to enable the polyglot re-use of components, allowing developers to specify how to use components written in various languages as libraries to fulfill dependencies in their Spin components as specified in the [Component Dependencies SIP](https://github.com/fermyon/spin/pull/2543). We’ll use this as the foundational work to then iterate on the polyglot development experience.

2. Building a composable runtime
   
   There are currently some assumptions baked into the Spin runtime about the functionality provided by underlying host environments, however it’s likely a host environment may offer a subset of the capabilities assumed in the Spin runtime today or an entirely custom set of capabilities. Modularizing the Spin runtime allows it to be more readily extendable and customizable based on host environment. This requires breaking changes and a major code refactor, so this work will also be the basis for a Spin 3.0 release. See the [Spin Factors SIP](https://github.com/fermyon/spin/pull/2518) for an overview.

3. Spin application development experience improvements
   
   It has become increasingly clear that there is interest in scenarios where Spin is used as a developer tool to build applications that target runtimes other than the Spin runtime (example: [NGINX Unit](https://unit.nginx.org/news/2024/fermyon-spin-rust-sdk/)). Right now, there are implicit assumptions made about the target environment both by the Spin CLI and the Spin SDKs. To support these scenarios, there will need to be work done around modularizing SDKs and building tooling that validates a Spin application can be run with a specific target environment. Foundational work for these efforts is described in the [Spin Build Target Check SIP](https://github.com/fermyon/spin/pull/2556) and is an ongoing effort.
