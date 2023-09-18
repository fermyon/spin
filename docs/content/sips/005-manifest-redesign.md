title = "SIP 005 - Application Manifest Redesign"
template = "main"
date = "2022-05-20T13:22:30Z"
---

Summary: A new design for Application Manifests (`spin.toml`)

Owner: <lann.martin@fermyon.com>

Created: May 20, 2022
Updated: Sep 11, 2023

## Background

Spin's Application Manifest file (usually named `spin.toml`) describes a Spin application's metadata,
components, and trigger configurations. The current design supports a single trigger type and combines
component and trigger configuration with an implicit one-to-one relationship:

```toml
spin_manifest_version = "1"
name = "hello-world"
trigger = { type = "http", base = "/hello" }

[[component]]
id = "hello"
# ...
[component.trigger]
route = "/hello"
```

In order to support future features, we would like to enable more flexible trigger configurations:

- Multiple trigger types in an application
  - an HTTP trigger that sends asynchronous jobs to a Redis trigger
- Multiple triggers associated with a single component
  - a file server component serving multiple HTTP trigger routes
- One trigger associated with multiple components
  - an HTTP trigger with middleware components

## Proposal

### Manifest version

Since this proposal represents a backward-incompatible change to the manifest format, the manifest
version changes.

> Note that this also changes the version from a string to an int, which is a minor/optional
> change for this proposal.

```toml
spin_manifest_version = 2
```

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

### Component config

Component IDs will become more important under this design, which we can emphasize
by moving the ID into the section header (aka TOML table key):

```toml
[component.hello-world]
# instead of id = "hello-world"
```

### Trigger config

In order to decouple triggers and components, we move triggers to new top-level
`[[trigger.<type>]]` sections:

```toml
[[trigger.http]]
route = "/hello"
handler = "hello"
[component.hello]
# ...
```

### Component inline config

In addition to being defined in their own top-level sections and referenced by ID,
component configs may be inlined; allowing this:

```toml
[[trigger.http]]
route = "/hello"
handler = { source = "hello.wasm" }
```

which would be equivalent to:

```toml
[[trigger.http]]
route = "/hello"
handler = "<generated-id>"
[component.<generated-id>]
source = "hello.wasm"
```

### Rename `config`

The Spin `config` feature is easily confused with Spin "Runtime Config". We can adopt
the `variables` terminology which is already used by the top-level `[variables]` section
and rename `[component.config]` to `[component.variables]`.

## Backward compatibility

Existing "version 1" manifests can be transformed into this "version 2" format losslessly:

```toml
trigger = { type = "http", base = "/" }
[[component]]
id = "hello"
source = "hello.wasm"
[component.trigger]
route = "/hello"
```

is equivalent to:

```toml
[application.trigger.http]
base = "/"
[[trigger.http]]
route = "/hello"
handler = "hello"
[component.hello]
source = "hello.wasm"
```

This upgrade could be performed automatically by `spin doctor`.

## Design options

### Remove http `base`

This trigger option is rarely used and has caused implementation headaches.
For any (rare) existing applications that do use it, the `spin doctor` upgrade
path could inline the `base` into the trigger `route`(s).
