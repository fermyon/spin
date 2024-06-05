title = "SIP 020 - Component Dependencies"
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

> ℹ️ NOTE: Throughout this document we refer to the concept of a registry. However, it is out of scope of this proposal to detail what that means. To read more about what is meant by registry see [The OpenContainers Distribution Spec](https://specs.opencontainers.org/distribution-spec/?v=v1.0.0).

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

## Dependency Names
Dependency names, as referenced above, have two forms: a plain kebab-case identifier (e.g. `my-name`) or a package pattern. The package pattern can be either the name of a package (e.g. `aws:client` or `aws:client@0.1.0`) or specify a concrete interface of a package (e.g. `aws:client/s3` or `aws:client/s3@0.1.0`). 

When using the package pattern form of dependency names in a component manifest's dependency section the set of names and versions used must not conflict. Two patterns conflict if they have overlapping interfaces and semver compatible versions. In otherwords the set of patterns must be disjoint when fully mapped to the underlying imports of the dependent component.

Some examples of conflicting dependency names:
* `aws:client` _conflicts_ with `aws:client@0.1.0`
* `aws:client` _conflicts_ with `aws:client/s3@0.1.0`
* `aws:client/s3@0.1.0` _conflicts_ with `aws:client/s3@0.1.1`

## Dependency Isolation
Statically composing the parent Spin component and its dependencies yields a component where the parent component and dependencies are no longer isolated from each other. Each dependency will inherit the configuration of the parent component, which may be desireable in some library use-cases (e.g., allowing the `aws:client/s3` dependency to make outbound requests via `allowed_outbound_hosts`). However, making this the default behavior contradicts the explicit opt-in nature of capabilities for Spin (and the component model), so by default, component dependencies in Spin will not inherit configuration.

To allow dependencies to inherit configuration from the parent Spin component, developers can set `dependencies_inherit_configuration = true`, as shown in the following example:

```toml
[component."infra-dashboard"]
# ...
allowed_outbound_hosts = ["https://s3.us-west-2.amazonaws.com"]
dependencies_inherit_configuration = true

[component."infra-dashboard".dependencies]
"aws:client" = "1.0.0"
```

The above configuration allows the component used to satisfy the `aws:client` dependency to make outbound HTTP requests to an S3 bucket 
specified by the host included in `allowed_outbound_hosts` (e.g. `https://s3.us-west-2.amazonaws.com/my-bucket/puppy.jpeg`). If `dependencies_inherit_configuration` is omitted or set to `false`, an error indicating the capability to make outbound HTTP requests is disabled, will be returned whenever the `aws:client` component attempts to make an outbound HTTP request.

### Configuration inheritance
At the time of writing this proposal, the following component-level configurations in a `Spin.toml` could be inherited by dependency components if inheritance is enabled:

* `allowed_outbound_hosts` 
* `key_value_stores`
* `variables`
* `ai_models`
* `files`
* `environment`


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

An alternative syntax could allow specifying, per dependency, which configurations to inerhit, e.g.:

```toml
[component."infra-dashboard"]
# ...
allowed_outbound_hosts = ["https://s3.us-west-2.amazonaws.com"]
key_value_stores = ["my-key-value-cache"]
dependencies_inherit_configuration = true

[component."infra-dashboard".dependencies]
"aws:client" = { version = "1.0.0", inherit = ["allowed_outbound_hosts"] }
```

To implement this more granular configuration inheritance, the monolithic `deny-all` adapter could be divided into atomic, configuration-specific adapters responsible for denying access to indivual pieces of configuration declared in the manifest (e.g. `deny-key-value-stores` `deny-variables`, `deny-environment`, etc.).