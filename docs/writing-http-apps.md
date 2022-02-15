# Writing HTTP applications using Spin

// TODO

Let's take the following Spin application. It sets a base path, `/test`, and
there are two components, each serving requests for `/test/hello/...` and
`/test/wagi/...` respectively:

```toml
name = "spin-hello-world"
trigger = { type = "http", base = "/test" }

[[component]]
source = "spin-module-that-prints-requests.wasm"
id = "hello"
[component.trigger]
route = "/hello/..."

[[component]]
source = "env_wagi.wasm"
id = "wagi"
[component.trigger]
route = "/wagi/..."
executor = "wagi"
```

Let's see how the application configuration above gets turned into the headers
by starting the application on `localhost:3000`.

First, let's send a request to the `hello` component.

```js
➜ curl 'localhost:3000/test/hello/abc/def?foo=bar' -d "abc"
Request {
    method: Method::Post,
    uri: "/test/hello/abc/def",
    headers: [
        (
            "host",
            "localhost:3000",
        ),
        (
            "user-agent",
            "curl/7.77.0",
        ),
        (
            "accept",
            "*/*",
        ),
        (
            "content-length",
            "3",
        ),
        (
            "content-type",
            "application/x-www-form-urlencoded",
        ),
        (
            "PATH_INFO",
            "/abc/def",
        ),
        (
            "X_FULL_URL",
            "http://localhost:3000/test/hello/abc/def?foo=bar",
        ),
        (
            "X_MATCHED_ROUTE",
            "/test/hello/...",
        ),
        (
            "X_BASE_PATH",
            "/test",
        ),
        (
            "X_RAW_COMPONENT_ROUTE",
            "/hello/...",
        ),
        (
            "X_COMPONENT_ROUTE",
            "/hello",
        ),
    ],
    params: [
        (
            "foo",
            "bar",
        ),
    ],
    body: Some(
        [
            97,
            98,
            99,
        ],
    ),
}
```

Available in the request object are the following fields:

- `method` — the HTTP method of the request — in this case, GET
- `uri` — the absolute path of the URI, _without_ the query parameters
- `params` — list of `(key, value)` pairs with the query parameters
- `headers` — list of `(key, value)` pairs with the headers (see the default
  headers for a list of default headers and their meaning)
- body — optional byte array containing the request body

Now let's send a request to the Wagi component and inspect the environment
variables:

```
➜ curl 'localhost:3000/test/wagi/abc/def?foo=bar' -d "abc"
### Arguments ###

### Env Vars ###
QUERY_STRING = foo=bar
REMOTE_HOST = 127.0.0.1
AUTH_TYPE =
X_FULL_URL = http://localhost:3000/test/wagi/abc/def?foo=bar
PATH_TRANSLATED = /abc/def
SERVER_PORT = 3000
X_MATCHED_ROUTE = /test/wagi/...
SERVER_PROTOCOL = HTTP/1.1
CONTENT_TYPE =
SERVER_SOFTWARE = WAGI/1
HTTP_HOST = localhost:3000
HTTP_ACCEPT = */*
REMOTE_ADDR = 127.0.0.1
X_RAW_COMPONENT_ROUTE = /wagi/...
CONTENT_LENGTH = 3
SERVER_NAME = localhost
GATEWAY_INTERFACE = CGI/1.1
HTTP_CONTENT_LENGTH = 3
HTTP_CONTENT_TYPE = application/x-www-form-urlencoded
X_BASE_PATH = /test
HTTP_USER_AGENT = curl/7.77.0
X_COMPONENT_ROUTE = /wagi
REMOTE_USER =
PATH_INFO = /abc/def
REQUEST_METHOD = POST
X_RAW_PATH_INFO = /abc/def
SCRIPT_NAME = /test/wagi

### STDIN ###
abc%
```

### The default headers

Spin sets a few default headers on the request based on the base path, component
route, and request URI, which should always be available when writing a module:

- `X_FULL_URL` - the full URL of the request —
  `http://localhost:3000/test/wagi/abc/def?foo=bar`
- `PATH_INFO` - the path info, relative to both the base application path _and_
  component route — in our example, where the base path is `/test`, and the
  component route is `/hello`, this is `/abc/def`.
- `X_MATCHED_ROUTE` - the base path and route pattern matched (including the
  wildcard pattern, if applicable) (this updates the header set in Wagi to
  include the base path) — in our case `"/test/hello/..."`.
- `X_RAW_COMPONENT_ROUTE` - the route pattern matched (including the wildcard
  pattern, if applicable) — in our case `/hello/...`.
- `X_COMPONENT_ROUTE` - the route path matched (stripped of the wildcard
  pattern) — in our case `/hello`
- `X_BASE_PATH` - the application base path — in our case `/test`.

Besides the headers above, components that use the Wagi executor also have
available
[all headers set by Wagi, following the CGI spec](https://github.com/deislabs/wagi/blob/main/docs/environment_variables.md).

### The HTTP headers
