title = "The Spin Redis trigger"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/redis-trigger.md"
---

Spin applications can be triggered by a new message on a [Redis channel](https://redis.io/topics/pubsub).
Spin will connect to a configured Redis instance and will invoke components for
new messages on the configured channels.

> See the [Rust language guide](/rust-components) for details on using Rust to
> build Redis components.

The Redis instance address is specified in the application trigger:

```toml
# spin.toml
trigger = { type = "redis", address = "redis://localhost:6379" }
```

> We are [exploring adding authentication for connecting to Redis](https://github.com/fermyon/spin/issues/192)
> and [adding host support for connecting to Redis databases](https://github.com/fermyon/spin/issues/181),
> from components, which would allow using the key/value store and publishing
> messages to channels.

Then, all components in the application are triggered when new messages are
published to channels in the instance. [Configuring](/configuration) the channel
 is done by setting the `channel` field in the component trigger configuration.

```toml
[component.trigger]
channel = "messages"
```

## The WebAssembly interface

The Redis trigger is built on top of the
[WebAssembly component model](https://github.com/WebAssembly/component-model).
The current interface is defined using the
[WebAssembly Interface (WIT)]((https://github.com/bytecodealliance/wit-bindgen/blob/main/WIT.md))
format, and is a function that takes the message payload as its only parameter:

```fsharp
// wit/ephemeral/spin-redis-trigger.wit

// The message payload.
type payload = list<u8>

// The entrypoint for a Redis handler.
handler: function(msg: payload) -> expected<_, error>
```

> The interface might change in the future to add the Redis instance and
> message channel as arguments to the function.

This is the function that all Redis components must implement, and which is
used by the Spin Redis executor when instantiating and invoking the component.
This interface (`spin-redis-trigger.wit`) can be directly used together with the
[Bytecode Alliance `wit-bindgen` project](https://github.com/bytecodealliance/wit-bindgen)
to build a component that the Spin HTTP executor can invoke.
This is exactly how [the Rust SDK for Spin](/rust-components) is built, and,
as more languages add support for the component model, how we plan to add
support for them as well.

> We are [exploring a compatibility layer (similar to Wagi)](https://github.com/fermyon/spin/issues/193)
> so languages that do not have support for the component model can be used to
> build Redis components for Spin.
