title = "SIP 008 - Key-Value Storage"
template = "main"
date = "2023-01-23T01:01:01Z"
---

Summary: Provide a generic interface for access to various key-value storage systems.

Owner(s): joel.dice@fermyon.com

Created: Jan 23, 2023

## Background

Spin should have native support for using a variety of key-value (KV) datastores.

Currently, users can use Redis for persistent key-value storage for Spin applications. This [documentation](https://developer.fermyon.com/cloud/data-redis.md) walks through how to use the existing Spin Redis SDK which leverages the `[outbound-redis.wit` interface](https://github.com/fermyon/spin/blob/main/wit/ephemeral/outbound-redis.wit). In contrast to the Redis interface, we are proposing a more general-purpose interface which can be used with a variety of implementations.

## Proposal

In order to support key-value stores, the following need to be added to Spin:

- A `WIT` file that defines the key-value interface
- SDK implementations for various programming languages
- A default local key-value store

Although not in scope for this proposal, we'll also want to expand the runtime configuration code added in [this PR](https://github.com/fermyon/spin/pull/798) to support configuring various key-value stores, including the default one.

### Key-Value Interface (`.wit`)

Spin should leverage the WebAssembly WASI subgroup’s work to define a universal key-value `WIT` interface. That work is taking place in [the `wasi-keyvalue` proposal](https://github.com/WebAssembly/wasi-keyvalue). However, the proposal was made assuming that `WIT` star imports are available, which is not yet the case. Therefore, we're modeling stores as pseudo-[resource handles](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#item-resource) which may be created using an `open` function and disposed using a `close` function.  Each operation on the store is a function which accepts a handle as its first parameter.

```
type store = u32

variant error {
  store-table-full,
  no-such-store,
  invalid-store,
  no-such-key,
  runtime(string)
}

open: func(name: string) -> expected<store, error>

get: func(store: store, key: string) -> expected<list<u8>, error>

set: func(store: store, key: string, value: list<u8>) -> expected<unit, error>

delete: func(store: store, key: string) -> expected<unit, error>

exists: func(store: store, key: string) -> expected<bool, error>

close: func(store: store)
```

*Note: the pseudo-resource design was inspired by the interface of similar functions in [WASI preview 2](https://github.com/bytecodealliance/preview2-prototyping/blob/d56b8977a2b700432d1f7f84656d542f1d8854b0/wit/wasi.wit#L772-L794).*

#### Built-in local key-value database

Spin will have a built-in database based on SQLite for testing, development, and some production use cases.  We’ve selected SQLite for this purpose because it’s easily embeddable, lightweight, and reliable.  It increases the Spin binary size by about 3% and the release build time by about 1.5%.  We’ve also considered lighter-weight options such as an in-memory hash map which is synced to a flat file on updates, but making this reliably atomic and durable would require reinventing the features SQLite already provides.
