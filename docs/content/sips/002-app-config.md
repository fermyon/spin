title = "SIP 002 - Application Configuration"
template = "main"
date = "2022-03-22T14:53:30Z"
---

Summary: A configuration system for Spin applications.

Owner: lann.martin@fermyon.com

Created: March 22, 2022

Updated: July 19, 2022

## Background

It is common for applications to require configuration at runtime that isn't known at build time or is too sensitive to be stored with build artifacts:

- Logging configuration
- Per-channel (production, staging, etc) service dependency URLs
- Database secrets

## Proposal

### Configuration schema is defined by components and applications

Configuration within a "parent" (component or application) consists of a number of configuration "slots":
- Slots are identified by a string "key"; in order to allow unambiguous conversion to environment variables or file paths, keys are constrained:
  - Keys must start with a letter (required for env vars)
  - Keys consist of only lowercase ascii alphanum and `_` (`[a-z0-9_]`)
    - Only one `_` at a time and not at the end (to allow delimiting in env vars with `__`)
- Slot keys must be unique within their parent, but to allow independent development of components different parents may have identical keys
- A slot must _either_ be marked as "required" _or_ must be given a default value
- A slot may be marked as "secret", in which case any associated value should be handled with care (e.g. not logged)

```toml
[variables]
required_key = { required = true }
optional_key = { default = "default_value" }
secret_key = { required = true, secret = true }
```

Default values can use template strings to reference other slots.
```toml
[variables]
key1 = { required = true }
key2 = { default = "prefix-{{ key1 }}-suffix" }
```

### Components and applications set configuration values of their dependencies

In dependency configuration, templates strings can reference top-level config keys (those in `[variables]`), "sibling" keys within the same dependency, and "ancestor" dependant configs.

- Top-level references use just the key name: `{{ top_level_key }}`
- "Sibling" references use a single `.` prefix: `{{ .sibling_key }}`
- "Ancestor" references use multiple `.`s: `{{ ..parent_dep_key }}`, `{{ ...grandparent_key }}`
- Circular / infinitely recursive references are not permitted

`spin.toml`:
```toml
[variables]
app_root = { default = "/app" }
log_file = { default = "{{ app_root }}/log.txt" }
...
[[component.config]]
work_root = "{{ app_root }}/work"      # -> "/app/work"
work_out = "{{ .work_root }}/output"   # -> "/app/work/output"
[[component.dependencies.dep1.config]]
dep_root = "{{ ..work_root }}/dep"     # -> "/app/work/dep"
```

### Configuration "providers" resolve application configuration

When resolving the value of an application configuration slot, providers will be queried in-order for a value. If no value is returned by any provider, the resolution will either use the default value or fail (if the slot is "required").

Provider configuration is handled by spin at instantiation time (`spin up`).

_Note: Provider configuration is TBD; as TOML it could look like:_

```toml
[[config-provider]]
type = "json_file"
path = "config.json"

[[config-provider]]
type = "env"
prefix = "MY_APP_"
```

#### Example providers

- Environment provider
  - Configured with a prefix, e.g. `SPIN_CONFIG_`
  - `key_one` -> `SPIN_CONFIG_KEY_ONE`
- File provider
  ```json
  {"key_one": "value-one"}
  ```
- Vault provider

### Configuration is exposed via component interface

`spin-config.wit`
```
// Unknown key is a runtime error
get-config: function(key: string) -> expect<string>
```
- Since each component gets its own instance of the `spin-config` import, the executor can resolve keys automatically and only expose a component's own config to it.

### Future design options

_This section contains possible future features which are not fully defined here._

#### Typed configuration

The above assumes only string values, but we could include some typing:
```toml
# Type can be inferred from default value:
number_key = { default = 123 }
# equivalent to:
number_key = { type = "number", default = 123 }

required_string = { type = "string", required = true }
# "bytes" would require e.g. base64 encoding in some places
encryption_key = { type = "bytes", required = true, secret = true}
```

#### WASI "configfs"

For languages without component support, we could expose config as synthetic mounted files:

```ruby
key1_value = File.read("/config/key1")

# Typed config; `.json` encodes values to JSON
key1_value = JSON.parse(File.read("/config/key1.json"))

# "bytes" type; `.raw` decodes from base64
encryption_key = File.read("/config/encryption_key.raw")
```
  