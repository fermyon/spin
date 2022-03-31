title = "SIP xxx - Application Configuration"
template = "main"
date = "2022-03-22T14:53:30Z"

---

Summary: A configuration system for Spin applications.

Owner: lann.martin@fermyon.com

Created: March 22, 2022

Updated: April 1, 2022

## Background

It is common for applications to require configuration at runtime that isn't known at build time or is too sensitive to be stored with build artifacts:

- Logging configuration
- Per-channel (production, staging, etc) service dependency URLs
- Database secrets

## Proposal

### Configuration is defined by components and applications

Configuration within a "parent" (component or application) consists of a number of configuration "slots":
- slots are uniquely identified within their parent by a string "key"; in order to allow unambiguous conversion to environment variables or file paths, keys are constrained:
  - must start with a letter (required for env vars)
  - consisting of only lowercase ascii alphanum and `-`
    - only one `-` at a time and not at the end (to allow separating env vars with `__`)
    - `-` matches WIT syntax but `_` might be more familiar to users
- a slot must _either_ be marked as "required" _or_ must be given a default value
- a slot may be marked as "secret", in which case any associated value should be handled with care (e.g. not logged)

```toml
[config]
# This simple form...
key1 = "default_value"
# ...is equivalent to:
key1 = { default = "default_value" }
# required & secret slot 
key2 = { required = true, secret = true }
```

Defaults can use template strings to reference other slots.
```toml
key1 = { required = true }
key2 = "prefix-{{ .key1 }}-suffix"
```

### Components and applications can set configuration of their direct dependencies

In dependency configuration, templates can reference the app config and "ancestor" dependant configs:

`spin.toml`:
```toml
[config]
app-root = "/app"
...
[[component.config]]
# Note: no '.'s needed when referencing top-level app config
work-root = "{{ app-root }}/work"  # -> "/app/work"
[[component.dependencies.dep1.config]]
dep-root = "{{ ..work-root }}/dep" # -> "/app/work/dep"
```

### Configuration "providers" resolve application configuration

When resolving the value of an application configuration slot, providers will be queried in-order for a value. If no value is returned by any provider, the resolution will either use the default value or fail (if the slot is "required").

Provider configuration is handled by spin at instantiation time (`spin up`).

_Note: Provider configuration is TBD; as TOML it could look like:_

```toml
[[config_provider]]
type = "json_file"
path = "config.json"

[[config_provider]]
type = "env"
prefix = "MY_APP_"
```

#### Example providers

- Environment provider
  - Configured with a prefix, e.g. `SPIN_APP_`
  - `key-one` -> `SPIN_APP_KEY_ONE`
- File provider
  ```json
  {"key-one": "value-one"}
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

#### Typed configuration

The above assumes only string values, but we could include some typing:
```toml
# Simple form is typed implicitly from its default value
string-key = "value"
string-key = { type = "string", default = "value" }

number-key = 123
number-key = { type = "number", default = 123 }

required-string = { type = "string", required = true }
# "bytes" would require e.g. base64 encoding in some places
encryption-key = { type = "bytes", required = true, secret = true}
```
- This would complicate the implementation but might be nice for users.

#### WASI "configfs"

For languages without component support, we could expose config as synthetic mounted files:

```ruby
key1-value = File.read("/config/key1")

# Typed config; `.json` encodes values to JSON
key1-value = JSON.parse(File.read("/config/key1.json"))

# "bytes" type; `.raw` decodes from base64
encryption-key = File.read("/config/encryption_key.raw")
```
  