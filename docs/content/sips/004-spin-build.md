title = "SIP 004 - The `spin build` command"
template = "main"
date = "2022-04-22T14:53:30Z"
---

Summary: A Spin command for building components locally.

Owner: radu@fermyon.com

Created: April 22, 2022

Updated: May 10, 2022

## Background

A Spin application is made up of one or more components. When developing a
multi-component application, it is very common to have multiple directories with
source code for components — and when making a changes to components, having to
manually go into the each component directory, compile the component, then go
back to the directory with `spin.toml` can be a very repetitive task.

## Proposal

This SIP proposes a new top level Spin command that would execute the `command`
field on the component configuration in a local `spin.toml` manifest file:

```toml
[[component]]
id = "hello"
source = "target/wasm32-wasi/release/spinhelloworld.wasm"

[[component.build]]
command = "cargo build --target wasm32-wasi --release"
```

The `spin build` command would execute the command set by each component, thus
building all components with a single command.

As of [#352](https://github.com/fermyon/spin/pull/352), the basic `spin build`
command described in this document has been implemented in Spin.

## Future design considerations

- do we need an application-level `build` section?
- do we need `pre` and `post` sections to execute before and after `command`?
- do we need the ability to set environment variables for build commands?
- do we need OS / arch specific commands? (`[[component.build.windows]]`)?
- setting a directory to watch for changes and re-build?
- option to automatically run `spin up` after building?
