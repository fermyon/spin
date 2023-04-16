title = "SIP 000 - Sqlite"
template = "main"
date = "2023-04-17:00:00Z"
---

Summary: Provide a generic interface for access to a sqlite databases

Owner(s): ryan.levick@fermyon.com

Created: Apr 17, 2023

## Background

Spin currently supports two database types: mysql and [postgres](https://developer.fermyon.com/cloud/data-postgres) which both require the user to provide their own database that is exposed to users through the SDK. 

In contrast to the these other interfaces, the sqlite implementation would easily allow local spin deployment to use a local sqlite database file, and it provides those hosting spin deployment envionments (e.g., Fermyon Cloud) to implement lightweight sqlite implementations.

### What about `wasi-sql`?

[`wasi-sql`](https://github.com/WebAssembly/wasi-sql) is a work-in-progress spec for a generic SQL interface that aims to support "the features commonly used by 80% of user application". It is likely that when `wasi-sql` is more mature users will be able to successfully use functionality based on the `wasi-sql` interface to interact with the sqlite databases. However, there are still reasons that a dedicated sqlite interface is still useful:

* For the 20% of use cases where `wasi-sql` is too generic a dedicated `sqlite` interface can provide that functionality. 
* The `wasi-sql` spec is under active investigation, and there are large questions about how to best support such a wide breadth of sql flavors. This implementation can help clarify those questions and push upstream work further along.

## Proposal

In order to support sqlite, the following need to be added to Spin:

- A `WIT` file that defines the sqlite interface
- SDK implementations for various programming languages
- A default local sqlite store (note: spin already uses sqlite for the KV implementation so this should be trivial)
- Potentially runtime configuration for configuring how sqlite is provisioned.

### Key-Value Interface (`.wit`)

We will start with the `wasi-sql` interface but deliberately change that interface as to better match sqlite semantics. This will ensure that we're not simply implementing early versions of the `wasi-sql` interface while still having good answers for why the interface differs when it does.

Like `wasi-sql` and the key-value store, we model resources such as database connections as pseudo-[resource handles](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#item-resource) which may be created using an `open` function and disposed using a `close` function.  Each operation on a connection is a function which accepts a handle as its first parameter.

Note that the syntax of the following `WIT` file matches the `wit-bindgen` version currently used by Spin, which is out-of-date with respect to the latest [`WIT` specification](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md) and implementation.  Once we're able to update `wit-bindgen`, we'll update the syntax of all the Spin `WIT` files, including this one.

```fsharp
TODO: copy over the wit file
```

*Note: the pseudo-resource design was inspired by the interface of similar functions in [WASI preview 2](https://github.com/bytecodealliance/preview2-prototyping/blob/d56b8977a2b700432d1f7f84656d542f1d8854b0/wit/wasi.wit#L772-L794).*

#### Implementation requirements

TODO: Open questions:
* Assumed sqlite version?

#### Built-in local database

By default, each app will have its own default database which is independent of all other apps. For local apps, the database will be stored by default in a hidden `.spin` directory adjacent to the app's `spin.toml`.  For remote apps: TODO

#### Granting access to components

By default, a given component of an app will _not_ have access to any database.  Access must be granted specifically to each component via the following `spin.toml` syntax:

```toml
sqlite_databases = ["<database 1>", "<database 2>", ...]
```

For example, a component could be given access to the default database using `sqlite_databases = ["default"]`.

### Runtime Config

Sqlite databases may be configured with `[sqlite_database.<database_name>]` sections in the runtime config file:

```toml
# The `default` config can be overridden
[sqlite_database.default]
path = ".spin/sqlite_key_value.db"
```

## Future work

TODO
