title = "SIP 013 - sqlite"
template = "main"
date = "2023-04-17:00:00Z"
---

Summary: Provide a generic interface for access to a sqlite databases

Owner(s): ryan.levick@fermyon.com

Created: Apr 17, 2023

## Background

Spin currently supports two database types: mysql and [postgres](https://developer.fermyon.com/cloud/data-postgres) which both require the user to provide their own database that is exposed to users through the SDK. This is largely a stopgap until sockets are supported in wasi and there is no longer a need for bespoke clients for these databases (users can bring their favorite client libraries instead).

In contrast to the these other interfaces, the sqlite implementation would easily allow local Spin deployment to use a local sqlite database file, and it provides those hosting Spin deployment envionments (e.g., Fermyon Cloud) to implement lightweight sqlite implementations. In short, a sqlite interface in Spin would allow for a "zero config" experience when users want to work with a SQL database.

### What about `wasi-sql`?

[`wasi-sql`](https://github.com/WebAssembly/wasi-sql) is a work-in-progress spec for a generic SQL interface that aims to support "the features commonly used by 80% of user application". It is likely that when `wasi-sql` is more mature users will be able to successfully use functionality based on the `wasi-sql` interface to interact with a sqlite databases. However, there are still reasons that a dedicated sqlite interface would still be useful:

* For the 20% of use cases where `wasi-sql` is too generic, a dedicated `sqlite` interface can provide that functionality. 
* The `wasi-sql` spec is under active investigation, and there are large questions about how to best support such a wide breadth of sql flavors. This implementation can help clarify those questions and push upstream work further along.

## Proposal

In order to support sqlite, the following need to be added to Spin:

- A `WIT` file that defines the sqlite interface
- SDK implementations for various programming languages
- A default local sqlite store (note: Spin already uses sqlite for the KV implementation so this should be trivial)
- Potentially runtime configuration for configuring how sqlite is provisioned.
- Potentially a mechansim for handling database migrations

### Interface (`.wit`)

We will start with the `wasi-sql` interface but deliberately change that interface as to better match sqlite semantics. This will ensure that we're not simply implementing early versions of the `wasi-sql` interface while still having good answers for why the interface differs when it does.

Like `wasi-sql` and the key-value store, we model resources such as database connections as pseudo-[resource handles](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#item-resource) which may be created using an `open` function and disposed using a `close` function. Each operation on a connection is a function which accepts a handle as its first parameter.

Note that the syntax of the following `WIT` file matches the `wit-bindgen` version currently used by Spin, which is out-of-date with respect to the latest [`WIT` specification](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md) and implementation. Once we're able to update `wit-bindgen`, we'll update the syntax of all the Spin `WIT` files, including this one.

```fsharp
// A handle to an open sqlite instance
type connection = u32

// The set of errors which may be raised by functions in this interface
variant error {
  // A database with the supplied name does not exist
  no-such-database,
  // The requesting component does not have access to the specified database (which may or may not exist).
  access-denied,
  // The provided connection is not valid
  invalid-connection,
  // The database has reached its capacity
  database-full,
  // Some implementation-specific error has occurred (e.g. I/O)
  io(string)
}

// Open a connection to a named database instance.
//
// If `database` is "default", the default instance is opened.
//
// `error::no-such-database` will be raised if the `name` is not recognized.
open: func(name: string) -> expected<connection, error>

// Execute a statement
execute: func(conn: connection, statement: string, parameters: list<value>) -> expected<unit, error>

// Query data
query: func(conn: connection, query: string, parameters: list<value>) -> expected<query-result, error>

// Close the specified `connection`.
close: func(conn: connection)

// A result of a query
record query-result {
  // The names of the columns retrieved in the query
  columns: list<string>,
  // The row results each containing the values for all the columns for a given row
  rows: list<row-result>,
}

// A set of values for each of the columns in a query-result
record row-result {
  values: list<value>
}

// The values used in statements/queries and returned in query results
variant value {
  integer(s64),
  real(float64),
  text(string),
  blob(list<u8>),
  null
}
```

*Note: the pseudo-resource design was inspired by the interface of similar functions in [WASI preview 2](https://github.com/bytecodealliance/preview2-prototyping/blob/d56b8977a2b700432d1f7f84656d542f1d8854b0/wit/wasi.wit#L772-L794).*

#### Interface open questions

**TODO**: answer these questions
* `row-result` can be very large. Should we provide some paging mechanism or a different API that allows for reading subsets of the returned data?
  * Crossing the wit boundary could potentially be expensive if the results are large enough. Giving the user control of how they read that data could be helpful.
* Is there really a need for query *and* execute functions since at the end of the day, they are basically equivalent?

#### Database migrations

Database tables typically require some sort of configuration in the form of database migrations to get table schemas into the correct state. To begin with a command line option supplied to `spin up` will be available for running any arbitrary SQL statements on start up and thus will be a place for users to run their migrations (i.e., `--sqlite "CREATE TABLE users..."`). It will be up to the user to provide idempotent statements such that running them multiple times does not produce unexpected results.

##### Future approaches

This CLI approach (while useful) is likely to not be sufficient for more advanced use cases. There are several alternative ways to address the need for migrations:
* Some mechanism for running spin components before others where the component receives the current schema version and decides whether or not to perform migrations. 
* The spin component could expose a current schema version as an exported value type so that an exported function would not need to called. If the exported schema version does not match the current schema version, an exported migrate function then gets called.
* A spin component that gets called just after pre-initialization finishes. Similarly, this component would expose a schema version and have an exported migration function called when the exported schema version does not match the current schema version.
* Configuration option in spin.toml manifest for running arbitrary SQL instructions on start up (e.g., `sqlite.execute = "CREATE TABLE users..."`)

It should be noted that many of these options are not mutually exclusive and we could introduce more than one (perhaps starting with one option that will mostly be replaced later with a more generalized approach).

For now, we punt on this question and only provide a mechanism for running SQL statements on start up through the CLI.

##### Alternatives

An alternative approach that was considered but ultimately reject was to require the user to ensure that the database is in the correct state each time their trigger handler function is run (i.e., provide no bespoke mechanism for migrations - the user only has access to the database when their component runs). There are a few issues with taking such an approach:
* Schema tracking schemes (e.g., a "migrations" table) themselves require some sort of bootstrap step.
* This goes against the design principle of keeping components handler functions simple and single purpose.

#### Implementation requirements

**TODO**: Open questions:
* Assumed sqlite version?
  * Semantics may change slightly depending on the sqlite version. It's unlikely that we'll be able to match the exact versions between whatever sqlite implementation spin users, Fermyon Cloud, and the user (if they decide to create their own databases manually). Having some guidance on which versions are expected to work might make it easier to guide the user down the right path.
* Capacity limits? The following are different capacities we might want to control:
  * The number of databases in total
  * The number of rows in a database
  * The size of certain row values (**question**: does sqlite or libsql impose any restrictions and do we just pass those on to the user?)

#### Built-in local database

By default, each app will have its own default database which is independent of all other apps. For local apps, the database will be stored by default in a hidden `.spin` directory adjacent to the app's `spin.toml`. For remote apps, the user should be able to rely on a default database as well. It is up to the implementor how this remote database is exposed (i.e., by having a sqlite database on disk or by using a third party network enabled database like [Turso](https://turso.tech)).

#### Granting access to components

By default, a given component of an app will _not_ have access to any database. Access must be granted specifically to each component via the following `spin.toml` syntax:

```toml
sqlite_databases = ["<database 1>", "<database 2>"]
```

For example, a component could be given access to the default database using `sqlite_databases = ["default"]`.

### Runtime Config

Sqlite databases may be configured with `[sqlite_database.<database_name>]` sections in the runtime config file:

```toml
# The `default` config can be overridden
[sqlite_database.default]
path = ".spin/some-other-database.db"

[sqlite_database.other]
path = ".spin/yet-another-database.db"
```

## Future work

In the future we may want to try to unify the three SQL flavors we currently have support for (sqlite, mysql, and postgres). This may not be desirable if it becomes clear that unifying these three (fairly different) SQL flavors actually causes more confusion than is worthwhile.
