title = "SIP 010 - Key-Value Storage"
template = "main"
date = "2023-01-23T01:01:01Z"
---

Summary: Provide a generic interface for access to various key-value storage systems.

Owner(s): joel.dice@fermyon.com

Created: Jan 23, 2023

Updated: Apr 5, 2023

## Background

Spin should have native support for using a variety of key-value (KV) datastores.

Currently, users can use Redis for persistent key-value storage for Spin applications. This [documentation](https://developer.fermyon.com/cloud/data-redis.md) walks through how to use the existing Spin Redis SDK which leverages the [`outbound-redis.wit` interface](https://github.com/fermyon/spin/blob/main/wit/ephemeral/outbound-redis.wit). In contrast to the Redis interface, we are proposing a more general-purpose interface which can be used with a variety of implementations.

## Proposal

In order to support key-value stores, the following need to be added to Spin:

- A `WIT` file that defines the key-value interface
- SDK implementations for various programming languages
- A default local key-value store

Although not in scope for this proposal, we'll also want to expand the runtime configuration code added in [this PR](https://github.com/fermyon/spin/pull/798) to support configuring various key-value stores, including the default one.

### Key-Value Interface (`.wit`)

Spin should leverage the WebAssembly WASI subgroup’s work to define a universal key-value `WIT` interface. That work is taking place in [the `wasi-keyvalue` proposal](https://github.com/WebAssembly/wasi-keyvalue). However, the proposal was made assuming that `WIT` star imports are available, which is not yet the case. Therefore, we're modeling stores as pseudo-[resource handles](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#item-resource) which may be created using an `open` function and disposed using a `close` function.  Each operation on the store is a function which accepts a handle as its first parameter.

Note that the syntax of the following `WIT` file matches the `wit-bindgen` version currently used by Spin, which is out-of-date with respect to the latest [`WIT` specification](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md) and implementation.  Once we're able to update `wit-bindgen`, we'll update the syntax of all the Spin `WIT` files, including this one.

```fsharp
// A handle to an open key-value store
type store = u32

// The set of errors which may be raised by functions in this interface
variant error {
  // Too many stores have been opened simultaneously. Closing one or more
  // stores prior to retrying may address this.
  store-table-full,

  // The host does not recognize the store name requested.  Defining and
  // configuring a store with that name in a runtime configuration file
  // may address this.
  no-such-store,

  // The requesting component does not have access to the specified store
  // (which may or may not exist).
  access-denied,

  // The store handle provided is not recognized, i.e. it was either never
  // opened or has been closed.
  invalid-store,

  // No key-value tuple exists for the specified key in the specified
  // store.
  no-such-key,

  // Some implementation-specific error has occurred (e.g. I/O)
  io(string)
}

// Open the store with the specified name.
//
// If `name` is the string "default", the default store is opened.
// Otherwise, `name` must refer to a store defined and configured in a
// runtime configuration file supplied with the application.
//
// `error::no-such-store` will be raised if the `name` is not recognized.
open: func(name: string) -> expected<store, error>

// Get the value associated with the specified `key` from the specified
// `store`.
//
// `error::invalid-store` will be raised if `store` is not a valid handle
// to an open store, and `error::no-such-key` will be raised if there is no
// tuple for `key` in `store`.
get: func(store: store, key: string) -> expected<list<u8>, error>

// Set the `value` associated with the specified `key` in the specified
// `store`, overwriting any existing value.
//
// `error::invalid-store` will be raised if `store` is not a valid handle
// to an open store.
set: func(store: store, key: string, value: list<u8>) -> expected<unit, error>

// Delete the tuple with the specified `key` from the specified `store`.
//
// `error::invalid-store` will be raised if `store` is not a valid handle
// to an open store.  No error is raised if a tuple did not previously
// exist for `key`.
delete: func(store: store, key: string) -> expected<unit, error>

// Return whether a tuple exists for the specified `key` in the specified
// `store`.
//
// `error::invalid-store` will be raised if `store` is not a valid handle
// to an open store.
exists: func(store: store, key: string) -> expected<bool, error>

// Return a list of all the keys in the specified `store`.
//
// `error::invalid-store` will be raised if `store` is not a valid handle
// to an open store.
get-keys: func(store: store) -> expected<list<string>, error>

// Close the specified `store`.
//
// This has no effect if `store` is not a valid handle to an open store.
close: func(store: store)
```

*Note: the pseudo-resource design was inspired by the interface of similar functions in [WASI preview 2](https://github.com/bytecodealliance/preview2-prototyping/blob/d56b8977a2b700432d1f7f84656d542f1d8854b0/wit/wasi.wit#L772-L794).*

#### Implementation requirements

In addition to the above interface, we specify a few additional implementation requirements which guest components may rely on.  At minimum, an conforming implementation must support:

- Keys as large as 256 bytes (UTF-8 encoded)
- Values as large as 1 megabyte
- Capacity for 1024 key-value tuples

Note, however, that an implementation is permitted to constrain overall store size irrespective of the above minimums, e.g. for cost reasons in a multitenant scenario.

#### Built-in local key-value database

Spin will have a built-in database based on SQLite for testing, development, and some production use cases.  We’ve selected SQLite for this purpose because it’s easily embeddable, lightweight, and reliable.  It increases the Spin binary size by about 3% and the release build time by about 1.5%.  We’ve also considered lighter-weight options such as an in-memory hash map which is synced to a flat file on updates, but making this reliably atomic and durable would require reinventing the features SQLite already provides.

By default, each app will have its own, persistent, default store which is independent of all other apps.  This could be implemented as a separate SQLite database for each app, a separate table for each app in a shared database, or even row-level separation via an `app` column in a shared table.  For the initial implementation, we'll use a separate database for each app.  For local apps, the database will be stored by default in a hidden `.spin` directory adjacent to the app's `spin.toml`.  For remote apps (e.g. served by `bindle`), an in-memory database will be used by default.

#### Granting access to components

By default, a given component of an app will _not_ have access to any key-value store.  Access must be granted specifically to each component via the following `spin.toml` syntax:

```toml
key_value_stores = ["<store 1>", "<store 2>", ...]
```

For example, a component could be given access to the default store using `key_value_stores = ["default"]`.

### Runtime Config

Key value stores may be configured with `[key_value_store.<store name>]` sections in the runtime config file:

```toml
# The `default` config can be overridden
[key_value_store.default]
type = "spin"
path = ".spin/sqlite_key_value.db"

# Example of a possible Redis-backed KV store type
[key_value_store.user_data]
type = "redis"
url = "redis://localhost"
```

## Future work

In addition to the built-in, SQLite-based implementation described above, we expect to add a number of other implementations backed by e.g. Redis, other relational databases, eventually consistent distributed stores, etc.  Each of these implementations will have its own performance, consistency, and durability characteristics, and some applications may use a combination of them to handle different types of data.

We also expect to add additional interfaces for e.g. atomic, bulk, and asynchronous operations, key expiration, etc., following [`wasi-keyvalue`](https://github.com/WebAssembly/wasi-keyvalue) as much as possible.

Since we're using SQLite by default, an app's database can be inspected and modified using the standard `sqlite3` CLI tool, as well as various GUI apps.  However, we may also want to add a `spin kv` subcommand which supports displaying and editing the store.
