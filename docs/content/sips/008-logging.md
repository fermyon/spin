title = "SIP 008 - Runtime Logging"
template = "main"
date = "2022-12-09T11:36:36Z"
---

Summary: Unified runtime logging

Owner: mail@etehtsea.me

Created: December 09, 2022

## Background

Spin runtime logging is an output produced by running `spin up` command.


## Current experience

By default, spin prints into `stdout` only the basic info:

```
$ spin up

Serving http://127.0.0.1:3000
Available Routes:
  hello: http://127.0.0.1:3000/hello
    A simple component that returns hello.
```

Application log is implicitly written into `~/.spin/<spin_app>/logs/<component_id>_std{err|out}.log`.
The log for the spin itself has log level `ERROR` and outputs into `stderr`.

To enable a full logging experience, a developer needs to provide `--follow-all` to duplicate app log into `stderr` and also `RUST_LOG=spin=<log_level>` to see the spin log in the `stderr`.


## Proposal

- Set the default log level to `INFO`.
  `INFO` log level is the most widely used across frameworks. It shouldn't make the output overly verbose, but
- Set default log destination to `stdout/stderr`.
- Provide a way for spin apps to emit log messages (besides using `println!`/`eprintln!`).
  *To consider*: unify WIT with [wasi-logging proposal][].
- Log `WARN`and `ERROR` into `stderr` and `INFO`, `DEBUG`, `TRACE` into `stdout`.
- Log level `OFF` disables logging completely.

### Goal

To provide the new baseline that gives foundation for further iterative development of new features and enhancements based on the feedback.

### Configuration

Command-line arguments:

```
--log-level <LOG_LEVEL>
    Describes the level of verbosity

    [default: INFO]
    [possible values: ERROR, WARN, INFO, DEBUG, TRACE, OFF]

--log-timezone <LOG_TIMEZONE>
    [possible values: local, utc]
```

### Format

Conforms to the default format of the `tracing_subscriber`: `<rfc_3339_datetime>  <log-level> <crate::module>: <text>`.

Severity levels:

- ERROR
- WARN
- INFO
- DEBUG
- TRACE

### Examples

```
spin/examples/http-rust $ spin up --log-level debug
2022-12-16T15:36:17.428429Z  INFO spin_http: Serving http://127.0.0.1:3000
2022-12-16T15:36:17.428429Z  INFO spin_http: Available Routes:
  hello: http://127.0.0.1:3000/hello
    A simple component that returns hello.
2022-12-16T15:36:21.897190Z  INFO spin_http: Processing request for application spin-hello-world on URI http://localhost:3000/hello
2022-12-16T15:36:21.897190Z  DEBUG spin_hello_world::hello Request headers: {"host": "localhost:3000", "user-agent": "curl/7.86.0", "accept": "*/*", "spin-path-info": "", "spin-full-url": "http://localhost:3000/hello", "spin-matched-route": "/hello", "spin-base-path": "/", "spin-raw-component-route": "/hello", "spin-component-route": "/hello"}
2022-12-16T15:36:21.898652Z  INFO spin_http::spin: Request finished, sending response with status code 200 OK
```


## Future design considerations

- Different prod/dev modes
  * Timezone - Dev: localtime; Prod: UTC
  * Severity - Dev: INFO; Prod: WARN

- Configuration through a config file

`spin.toml`:
```toml
[variables]
log = {
  level: "INFO",
  timezone: "local"
}
```

Per component configuration:

`spin.toml`:
```toml
[[component]]
id = "hello"
log = {
  level: "OFF",
}
```

- Output format customization
  It might be JSON or some other format.

- Log format customization
  A user might want to change log record format or datetime format.


[wasi-logging proposal]: https://github.com/WebAssembly/wasi-logging
