title = "SIP 003 - Trigger Executors"
template = "main"
date = "2022-04-29T14:53:30Z"
---

Summary: A new architecture for Spin trigger executors.

Owner: lann.martin@fermyon.com

Created: April 29, 2022

Updated: May 5, 2022

## Background

Spin currently includes built-in implementations for two trigger types: HTTP, and Redis (PubSub).
We want to allow for additional trigger types in the future, whether they are "internal" types developed and
supported within the Spin codebase, or "external" types developed by the broader community.

While it is currently possible to build external trigger types - as in the
[TimerTrigger example](https://github.com/fermyon/spin/blob/main/examples/spin-timer/src/main.rs) - it requires replicating much of the
functionality provided by the `spin up` command.

## Proposal

### Trigger types are implemented as "trigger executors"

These executors are regular subcommands/binaries, but are not normally executed directly by
users.

* "Internal" triggers use `spin` subcommands, e.g. `spin trigger http`
* "External" triggers use external binaries, e.g. `spin-trigger-timer`
* Trigger executors share common code, including common CLI options in a `spin-trigger` crate

### The `spin up` command changes

It remains the primary entrypoint for starting a Spin app, but its behavior is changed to:

1. Load the application manifest
1. Find a trigger executor for the application (e.g. check the `PATH` for `"spin-trigger-${trigger.type}"`)
1. Set some environment variables:
   * `SPIN_MANIFEST_URL`: a URL pointing to the application manifest (e.g. `file:///path/to/spin.toml` or `bindle+https://bindle-server?id=bindle-id`)
   * `SPIN_TRIGGER_TYPE` the spin trigger type (e.g. `http`) being executed
1. `exec` the trigger executor, forwarding its own CLI arguments

## Future design options

_This section contains possible future features which are not fully defined here._

### Multi-trigger orchestration

If multiple trigger types were to be supported by a single application, the role of `spin up` could
grow to include trigger orchestration:

* Finding executors for _all_ required trigger types
* Forking a child process for each trigger executor
* Managing the child processes (restarts, graceful shutdowns, etc.)

## Potential disadvantages

If the "Multi-trigger orchestration" option is taken, having separate processes for different triggers
might have some disadvantages:

* Multiple triggers types would use more system resources implemented as multiple processes rather than as
  multiple async tasks in the same process, especially if a single module implements multiple trigger type
  handlers.
* There has been discussion the possibility of trigger handlers making optimized "in-memory" calls to
  other triggers. This design would make cross-trigger-type calls of this sort more complex at best.
