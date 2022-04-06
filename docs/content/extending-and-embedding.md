title = "Extending and embedding Spin"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/extending-and-embedding.md"
---

> The complete example for extending and embedding Spin [can be found on GitHub](https://github.com/fermyon/spin/tree/main/examples/spin-timer).

Spin currently implements triggers and application models for:

- [HTTP applications](/http-trigger) that are triggered by incoming HTTP
requests, and that return an HTTP response
- [Redis applications](/redis-trigger) that are triggered by messages on Redis
channels

The Spin internals and execution context (the part of Spin executing
components) are agnostic of the event source and application model.
In this document we will explore how to extend Spin with custom event sources
(triggers) and application models built on top of the WebAssembly component
model, as well as how to embed Spin in your application.

In this article we will build a Spin trigger to run the applications based on a
timer, executing Spin components at configured time interval.

The current application types that can be implemented with Spin have entry points
defined using
[WebAssembly Interface (WIT)]((https://github.com/bytecodealliance/wit-bindgen/blob/main/WIT.md)):

```fsharp
// The entry point for an HTTP handler.
handle-http-request: function(req: request) -> response

// The entry point for a Redis handler.
handle-redis-message: function(msg: payload) -> expected<_, error>
```

The entry point we want to execute for our timer trigger takes a string as its
only argument (the trigger will populate that with the current date and time),
and it expects a string as the only return value. This is purposefully chosen
to be a simple function signature:

```fsharp
// examples/spin-timer/spin-timer.wit
handle-timer-request: function(msg: string) -> string
```

This is the function that all components executed by the timer trigger must
implement, and which is used by the timer executor when instantiating and
invoking the component.

Let's have a look at building the timer trigger:

```rust
// examples/spin-timer/src/main.rs
wit_bindgen_wasmtime::import!("spin-timer.wit");
type ExecutionContext = spin_engine::ExecutionContext<spin_timer::SpinTimerData>;

/// A custom timer trigger that executes a component on every interval.
#[derive(Clone)]
pub struct TimerTrigger {
    /// The interval at which the component is executed.
    pub interval: Duration,
    /// The Spin execution context.
    engine: Arc<ExecutionContext>,
}
```

A few important things to note from the start:

- we use the WIT defined entry point with the
[Bytecode Alliance `wit-bindgen` project](https://github.com/bytecodealliance/wit-bindgen)
to generate "import" bindings based on the entry point — this generates code that
allows us to easily invoke the entry point from application components that
implement our new application model.
- the new trigger has a field that contains a `Application<CoreComponent>` —
in most cases, either `CoreComponent` will have to be updated with new trigger
and component configuration (not the case for our simple application model),
or an entirely new component can be defined and used in `Application<T>`.
- the trigger has a field that contains the Spin execution context — this is the
part of Spin that instantiates and helps execute the WebAssembly modules. When
creating the trigger (in the `new` function, you get access to the underlying
Wasmtime store, instance, and linker, which can be configured as necessary).

Finally, whenever there is a new event (in the case of our timer-based trigger
every `n` seconds), we execute the entry point of a selected component:

```rust
/// Execute the first component in the application manifest.
async fn handle(&self, msg: String) -> Result<()> {
    // create a new Wasmtime store and instance based on the first component's WebAssembly module.
    let (mut store, instance) =
        self.engine
            .prepare_component(&self.app.components[0].id, None, None, None, None)?;

    // spawn a new thread and call the entry point function from the WebAssembly module 
    let res = spawn_blocking(move || -> Result<String> {
            // use the auto-generated WIT bindings to get the Wasm exports and call the `handle-timer-request` function.
        let t = spin_timer::SpinTimer::new(&mut store, &instance, |host| {
            host.data.as_mut().unwrap()
        })?;
        Ok(t.handle_timer_request(&mut store, &msg)?)
    })
    .await??;
    // do something with the result.
    log::info!("{}\n", res);
    Ok(())
}
```

A few notes:

- `prepare_component` is a function implemented by the Spin execution context,
and it handles taking the Wasmtime pre-instantiated module, mapping all the
component files, environment variables, and allowed HTTP domains, populating
the Wasmtime store with the appropriate data, and returning the store and instance.
- invoking the entry point `handle-timer-request` is done in this example in a new Tokio thread —
this is an implementation choice based on the needs of the trigger.
- the return value from the component (a string in this example) can then be
used — in the case of the HTTP trigger, this is an HTTP response, which is then
returned to the client.

This is very similar to how the [HTTP](/http-trigger) and [Redis](/redis-trigger)
triggers are implemented, and it is the recommended way to extend Spin with your
own trigger and application model.

Writing components for the new trigger can be done by using the
[`wit-bindgen` tooling](https://github.com/bytecodealliance/wit-bindgen) from
Rust and other supported languages (see [the example in Rust](https://github.com/fermyon/spin/tree/main/examples/spin-timer/example)):

```rust
// automatically generate Rust bindings that help us implement the 
// `handle-timer-request` function that the trigger will execute.
wit_bindgen_rust::export!("../spin-timer.wit");
...
fn handle_timer_request(msg: String) -> String {
    format!("ECHO: {}", msg)
}
```

Components can be compiled to WebAssembly, then used from a `spin.toml`
application manifest.

Embedding the new trigger in a Rust application is done by creating a new trigger
instance, then calling its `run` function:

```rust
// app() is a utility function that generates a complete application configuration.
let trigger = TimerTrigger::new(Duration::from_secs(1), app()).await?;
// run the trigger indefinitely
trigger.run().await
```

> We are exploring [APIs for embedding Spin from other programming languages](https://github.com/fermyon/spin/issues/197)
> such as Go or C#.

In this example, we built a simple timer trigger — building more complex triggers
would also involve updating the Spin application manifest, and extending
the application-level trigger configuration, as well as component-level
trigger configuration (an example of component-level trigger configuration
for this scenario would be each component being able to define its own
independent time interval for scheduling the execution).

## Other ways to extend and use Spin

Besides building custom triggers, the internals of Spin could also be used
independently:

- the Spin execution context can be used entirely without a `spin.toml`
application manifest — for embedding scenarios, the configuration for the
execution can be constructed without a `spin.toml` (see [issue #229](https://github.com/fermyon/spin/issues/229)
for context)
- the standard way of distributing a Spin application can be changed by
re-implementing the [`loader`](https://github.com/fermyon/spin/tree/main/crates/loader)
and [`publish`](https://github.com/fermyon/spin/tree/main/crates/publish) crates —
all is required is that loading the application returns a valid
`Application<CoreComponent>` that the Spin execution context can use to
instantiate and execute components.
