title = "SIP 005 - Application Manifest Redesign"
template = "main"
date = "2022-05-20T13:22:30Z"
---

Summary: A new design for Application Manifests (`spin.toml`)

Owner: lann.martin@fermyon.com

Created: May 20, 2022

## Background

Spin's Application Manifest file (usually named `spin.toml`) describes a Spin application's metadata,
components, and trigger configurations. The current design supports a single trigger type and combines
component and trigger configuration with an implicit one-to-one relationship:

```toml
name = "hello-world"
trigger = { type = "http", base = "/hello" }

[[component]]
id = "world"
# ...
[component.trigger]
route = "/world"
```

In order to support future features, we would like to enable more flexible trigger configurations:

- Multiple trigger types in an application
  - an HTTP trigger that sends asynchronous jobs to a Redis trigger
- Multiple triggers associated with a single component
  - a file server component serving multiple HTTP trigger routes
- One trigger associated with multiple components
  - an HTTP trigger with middleware components

## Proposal

### Global trigger config

In order to allow for multiple trigger types, we remove the top-level `trigger.type` field.
We also move all application metadata from the top-level config to a new `[application]`
section, and all "global trigger config" to new `[application.trigger.<type>]` sections:

```toml
[application]
name = "hello-world"
[application.trigger.http]
base = "/hello"
```

> In the short term we can continue to enforce the use of exactly one trigger type in an
> application as a manifest validation step.

### Trigger config

In order to decouple triggers and components, we move triggers to new top-level
`[[trigger.<type>]]` sections:

```toml
[[trigger.http]]
route = "/world"
component = "world"

[[component]]
id = "world"
# ...
```

### Manifest version

Since this proposal represents a backward-incompatible change to the manifest format, the manifest
version changes. The existing `spin_version = "1"` field name leaves some room for misinterpretation
as "compatible with Spin project version 1". As an optional part of this proposal, change that name:

```toml
spin_manifest_version = "2"
```

## Backward compatibility

Existing "version 1" manifests can be transformed into this "version 2" data structure losslessly:

```toml
trigger = { type = "http", base = "/" }
[[component]]
id = "hello"
[component.trigger]
route = "/hello"
```

is equivalent to:

```toml
[application.trigger.http]
base = "/"
[[trigger.http]]
route = "/hello"
component = "hello"
[[component]]
id = "hello"
```

## Design options

### Don't update the manifest version

Instead of updating the manifest version we could not do that. It would be nice to include
error message guidance and/or a `spin doctor` tool to upgrade the manifest format. This
would be easy to detect if we switch the `spin_version` key.

### Trigger IDs

We might want to be able to reference individual triggers, for example to implement a
`spin up --trigger=world`. It would save some compatibility headache if we require IDs now:

```toml
[[trigger.http]]
id = "world"
```

Mandatory IDs would require a translation from the current "v1" manifest, which could either
simply copy the component ID (if we're fine with component and trigger IDs being separate
namespaces), or e.g. add a suffix to the component ID (`world` -> `world-trigger`), which
could technically cause conflicts but doesn't seem likely in practice.
