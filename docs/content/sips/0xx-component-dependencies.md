title = "SIP 0XX - Component Dependencies"
template = "main"
date = "2024-05-31T12:00:00Z"
---
Summary: This feature enables developers to specify and manage dependencies for Spin components, allowing the use of components from various languages and sources, either locally or remotely, with configurable inheritance of parent component configuration.

Owner(s): [brian.hardock@fermyon.com](mailto:brian.hardock@fermyon.com)

Created: May 31, 2024

# Background
This feature enables the polyglot re-use of components, allowing developers to specify how to use components written in various languages as libraries to fulfill dependencies in their Spin components.

For instance, consider writing a Spin component in Go that depends on the `aws:client/s3` interface (expressed as an import in the compiled Go component, e.g., `import aws:client/s3`). To satisfy this dependency, you can use a componentized S3 client written in Rust that exports an instance of the `aws:client/s3` interface. Using the proposed syntax in this SIP, Spin developers can instruct Spin on how to compose these components together to achieve this.

# Proposal
Developers can use the following syntax to specify that they want to use a component as a dependency, either from a local disk or via a remote component registry:

## Using dependencies from disk (local)
```toml
[component."infra-dashboard".dependencies]
# Components can satisfy dependencies by import name (e.g., "aws:client/s3")
# using a component from disk. It is implicitly assumed that the
# `my_aws_client.wasm` component exports an instance of the
# `aws:client/s3` interface.
"aws:client/s3" = { path = "my_aws_client.wasm" }

# Optionally, explicitly specify the export name to use to satisfy the `aws:client/s3` interface.
"aws:client/s3" = { path = "my_aws_client.wasm", export = "my-s3-client" }

# Without an interface name, attempt to resolve every import of the "aws:client" package.
"aws:client" = { path = "my_aws_client.wasm" }
```

> ⚠️ NOTE: Using the package and interface forms together (e.g. `aws:client` and `aws:client/s3` respectively) within the same dependencies section will result in an error to prevent resolution ambiguities.

## Using dependencies from a registry (remote)
```toml
[component."infra-dashboard".dependencies]
# Equivalent to { version = "1.0.0" , package = "aws:client"}
"aws:client" = "1.0.0"

# Use the `aws:client` component package to satisfy any number of "wasi:blobstore" imports ...
"wasi:blobstore" = { registry = "my-registry.io", version = "0.1.0", package = "aws:client" }
```

## Dependency Isolation
Statically composing the parent Spin component and its dependencies yields a component where the parent component and dependencies are no longer isolated from eachother. Each dependency will inherit the configuration of the parent component, which may be desireable in some library use-cases (e.g., allowing the `aws:client/s3` dependency to make outbound requests via `allowed_outbound_hosts`). However, making this the default behavior contradicts the explicit opt-in nature of capabilities for Spin (and the component model), so by default, component dependencies in Spin will not inherit configuration.

To allow dependencies to inherit configuration from the parent Spin component, developers can set `dependencies_inherit_configuration = true`, as shown in the following example:

```toml
[component."infra-dashboard"]
# ...
allowed_outbound_hosts = ["https://s3.us-west-2.amazonaws.com/my-bucket/puppy.jpeg"]
dependencies_inherit_configuration = true

[component."infra-dashboard".dependencies]
"aws:client" = "1.0.0"
```

The above configuration allows the component used to satisfy the `aws:client` dependency to make outbound HTTP requests to the S3 bucket specified in `allowed_outbound_hosts`. If `dependencies_inherit_configuration` is omitted or set to `false`, an `AccessDenied` error will be returned whenever the `aws:client` component attempts to make an outbound HTTP request.

### The `spin-deny-all` adapter
To implement the default behavior (i.e. `dependencies_inherit_configuration = false`), Spin will apply what's called the `spin-deny-all` adapter to maintain isolation between components. When configuration inheritance is disabled, Spin will compose the ahead-of-time constructed deny adapter with each dependency, resulting in a new component that denies access to various capabilities (e.g. `wasi:http/outgoing-handler`, `fermyon:spin/variables`, etc.).

The `spin-deny-all` adapter is a component that targets the following world:

```
package fermyon:spin-virt;

world deny-all {
    # WASI
    export wasi:cli/environment@0.2.0;
    export wasi:filesystem/preopens@0.2.0;
    export wasi:http/outgoing-handler@0.2.0;
    export wasi:sockets/ip-name-lookup@0.2.0;
    export wasi:sockets/tcp@0.2.0;
    export wasi:sockets/udp@0.2.0;
    
    # Spin
    export fermyon:spin/llm@2.0.0;
    export fermyon:spin/redis@2.0.0;
    export fermyon:spin/mqtt@2.0.0;
    export fermyon:spin/rdbms-types@2.0.0;
    export fermyon:spin/postgres@2.0.0;
    export fermyon:spin/mysql@2.0.0;
    export fermyon:spin/sqlite@2.0.0;
    export fermyon:spin/key-value@2.0.0;
    export fermyon:spin/variables@2.0.0;
}
```

#### Future work
A future iteration of this proposal could enable more fine-grained configuration inheritance, allowing developers to toggle which specific configurations can be inherited by dependencies. The following syntax could allow the component used to satisfy the `aws:client` dependency to inherit `allowed_outbound_hosts` while denying access to `my-key-value-cache` declared in the `key_value_stores` configuration:

```toml
[component."infra-dashboard"]
# ...
allowed_outbound_hosts = ["https://s3.us-west-2.amazonaws.com/my-bucket/puppy.jpeg"]
key_value_stores = ["my-key-value-cache"]
dependencies_inherit_configuration = ["allowed_outbound_hosts"]

[component."infra-dashboard".dependencies]
"aws:client" = "1.0.0"
```

To implement this more granular configuration inheritance, the monolithic `deny-all` adapter could be divided into atomic, configuration-specific adapters responsible for denying access to indivual pieces of configuration declared in the manifest (e.g. `deny-key-value-stores` `deny-variables`, `deny-environment`, etc.).