title = "Configuration for Spin applications"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/configuration.md"
---

Spin applications are comprised of general information (metadata), and a collection
of at least one _component_. Configuration for a Spin application lives in a TOML
file called `spin.toml` (the _application manifest_). In the example below we can see
a simple HTTP application with a single component executed when the `/hello` endpoint
is accessed:

```toml
spin_version = "1"
name = "spin-hello-world"
description = "A simple application that returns hello world."
trigger = { type = "http", base = "/" }
version = "1.0.0"

[[component]]
id = "hello"
source = "target/wasm32-wasi/release/spinhelloworld.wasm"
[component.trigger]
route = "/hello"
```

## Configuration reference

### Application manifest

The following are the fields supported by the `spin.toml` manifest file:

- `spin_version` (REQUIRED): Spin API version. Currently, this value MUST be
  `"1"`.
- `name` (REQUIRED): Name of the application.
- `version` (REQUIRED): Version of the application.
- `description` (OPTIONAL): Description of the application.
- `authors` (OPTIONAL): List with the authors of the application.
- `trigger` (REQUIRED): Trigger for the application. Currently, the two
implemented trigger types are:
  - `http`: All components of the application are invoked as a result of
  incoming HTTP requests. [The HTTP trigger](/http-trigger) configuration has
  the following fields:
    - `type` (REQUIRED): The application trigger type with the value `"http"`.
    - `base` (REQUIRED): The base path for the HTTP application which will be
      prepended to the routes of all components. (For example, if `base = "/foo"`
      and a component has `route = "/bar"`, the component will be invoked for
      requests on `/foo/bar`.)
  - `redis`: All components of the application are invoked as a result of messages
being published on the queues of Redis instance. [The Redis trigger](/redis-trigger)
configuration has the following fields:
    - `type` (REQUIRED): The application trigger type with the value `"redis"`.
    - `address` (REQUIRED): The address of the Redis instance the components
are using for message subscriptions.
- a list of `component` objects (REQUIRED) defining the application components.

### Component configuration

Each `component` object has the following fields:

- `id` (REQUIRED): unique (per application) ID of the component, used at runtime
  to select between multiple components of the same application.
- `source` (REQUIRED): Source for the WebAssembly module of the component. This
  field can be _one_ the following:
  - a string with the path to a local file containing the WebAssembly module for
    the component OR
  - a pair of `reference` (REQUIRED) and `parcel` (REQUIRED) fields pointing to
    a remote bindle package
    ([Planned in #135](https://github.com/fermyon/spin/issues/135)).
- `environment` (OPTIONAL): Environment variables to be made available inside
  the WebAssembly module at runtime.
- `files` (OPTIONAL): Files to be made available inside the WebAssembly module
  at runtime. This is a list, each element of which is either:
  - a file path or glob relative to the `spin.toml` file (for example
    `file.txt`, or `content/static/**/*`) OR
  - a mapping of a `source` (REQUIRED), a directory relative to `spin.toml` and
    `destination` (REQUIRED), the absolute mount path to be mapped inside the
    WebAssembly module. For example
    `{ source = "content/", destination = "/"}`.
- `allowed_http_hosts` (OPTIONAL): List of HTTP hosts the component is allowed
  to make HTTP requests to (using the
  [WASI experimental HTTP library](https://github.com/deislabs/wasi-experimental-http))
- `trigger` (REQUIRED): Trigger configuration for the component. Triggers are
  the components that generate events that cause the execution of components.
  The trigger configuration for a component must be compatible with the top-level
  trigger type of the application. As such, there are two possible trigger
  configurations for components, HTTP or Redis:
  - `http`: The configuration for an HTTP component. This has the following fields:
    - `route` (REQUIRED): The HTTP route the component will be invoked for. It can
      either be an exact route (for example `/foo/test`), or it can contain a
      wildcard (`/foo/test/...`) as the last path segment, which means the
      component will be invoked for every request starting with the `/foo/test`
      prefix (for example `/foo/test/abc/def`).
    - `executor` (REQUIRED): The executor for the HTTP component. There are
      currently two executor `type`s:
      - `spin` (DEFAULT): the Spin HTTP executor, which uses
        [the WebAssembly component model](https://github.com/WebAssembly/component-model)
        OR
      - `wagi`: the Wagi CGI executor, which can be used to write components in
        any language that compiles to WASI. The Wagi executor has the following
        optional fields:
        - `argv` (OPTIONAL): The string representation of the `argv` list that
          should be passed into the handler. `${SCRIPT_NAME}` will be replaced
          with the script name, and `${ARGS}` will be replaced with the query
          parameters of the request, formatted as arguments. The default is to
          follow the CGI specification, and pass `${SCRIPT_NAME} ${ARGS}`
        - `entrypoint` (OPTIONAL): The name of the function that should be called
          as the entry point to this handler. By default, it is `_start` (which in
          most languages translates to calling `main` in the guest module).
  - `redis`: The configuration for a Redis component. This has the following fields:
    - `channel` (REQUIRED): The Redis channel for which, whenever a new message
is published, the component will be invoked.

## Examples

- a Spin HTTP component that contains the files in `static/` mapped to `/`:

```toml
[[component]]
source = "modules/spin_static_fs.wasm"
id = "fileserver"
files = [ { source = "static/", destination = "/" } ]
[component.trigger]
route = "/static/..."
```

- a Wagi HTTP component that contains file mounts and sets the module `argv` and
  invokes a custom export function as the entry point:

```toml
[[component]]
source = "modules/env_wagi.wasm"
id = "env"
files = [ "content/**/*" , "templates/*", "scripts/*", "config/*"]
[component.trigger]
route = "/..."
executor = { type = "wagi", argv = "test ${SCRIPT_NAME} ${ARGS} done", entrypoint = "some-other-export-function" }
```

- a Redis component that is invoked for new messages on the `messages` channel:

```toml
[[component]]
id = "echo-message"
source = "spinredis.wasm"
[component.trigger]
channel = "messages"
```
