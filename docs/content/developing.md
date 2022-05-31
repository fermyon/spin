title = "Developing Spin applications"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/developing.md"
---

The Spin CLI offers a few commands to simplify developing applications.

## Building Spin applications

A Spin application is made up of one or more components. When developing a
multi-component application, it is very common to have multiple directories with
source code for components — and when making changes to components, having to
manually go into the each component directory, compile the component, then go
back to the directory with `spin.toml` can be a very repetitive task.

This is why Spin has a top-level command that will execute the build command
set by each component, `spin up`:

```toml
[component.build]
command = "cargo build --target wasm32-wasi --release --manifest-path http-rust/Cargo.toml"
```

Then, running `spin build` will execute, sequentially, each build command:

```
$ RUST_LOG=spin=trace spin build
2022-04-25T03:01:56.721630Z  INFO spin_build: Executing the build command for component rust-hello.
    Finished release [optimized] target(s) in 0.05s
2022-04-25T03:01:56.832360Z  INFO spin_build: Executing the build command for component rust-static-assets.
    Finished release [optimized] target(s) in 0.02s
2022-04-25T03:01:56.905424Z  INFO spin_build: Executing the build command for component rust-outbound-http.
    Finished release [optimized] target(s) in 0.02s
```

The `spin build` command is intended to offer a built-in way to build more complex
Spin applications without needing a separate build process.
It is not intended to replace complex build scripts — if
you have existing automated ways for building source code, those can be used
instead, or the build command can call that process.

`spin build --up` can be used to start the application after the build process
finishes for all application components.

## Component `workdir`

By default, the `command` to build a component is executed in the manifest's
directory. This can be changed. For example, assume a component is located in
subdirectory `deep`:

```bash
.
├── deep
│   ├── Cargo.toml
│   └── src
│       └── lib.rs
└── spin.toml
```

To run the build `command` in directory `deep`, set the component's `workdir`:

```toml
[component.build]
command = "cargo build --target wasm32-wasi --release"
workdir = "deep"
```

Note that `workdir` must be a relative path and it operates relative to the
`spin.toml`. Specifying an absolute path leads to an error.
