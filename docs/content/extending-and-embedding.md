title = "Extending and embedding Spin"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/extending-and-embedding.md"
---

> The complete example for extending and embedding Spin [can be found on GitHub](https://github.com/fermyon/spin/tree/main/examples/spin-timer-echo).

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

The current application types that can be implemented with Spin have entrypoints
defined using
[WebAssembly Interface (WIT)]((https://github.com/bytecodealliance/wit-bindgen/blob/main/WIT.md)):

```fsharp
// The entrypoint for an HTTP handler.
handle-http-request: function(req: request) -> response

// The entrypoint for a Redis handler.
handle-redis-message: function(msg: payload) -> expected<_, error>
```

Let's see how define a new application entrypoint for Spin:

```fsharp
// examples/spin-timer-echo/echo.wit
echo: function(msg: string) -> string
```

This is the function signature that all "echo" components must implement, and
which is used by the "echo" executor when instantiating and invoking the
component.

Let's define a new trigger for our new application type — a timer-based trigger:

```rust
// examples/spin-timer-echo/src/lib.rs
wit_bindgen_wasmtime::import!("examples/spin-timer-echo/echo.wit");

type ExecutionContext = spin_engine::ExecutionContext<echo::EchoData>;

/// A custom timer trigger that executes the
/// first component of an application on every interval.
#[derive(Clone)]
pub struct TimerTrigger {
    /// The interval at which the component is executed.
    pub interval: Duration,
    /// The application configuration.
    app: Configuration<CoreComponent>,
    /// The Spin execution context.
    engine: Arc<ExecutionContext>,
}
```

A few important things to note from the start:

- we use the WIT defined entrypoint with the
[Bytecode Alliance `wit-bindgen` project](https://github.com/bytecodealliance/wit-bindgen)
to generate "import" bindings based on the entrypoint — this generates code that
allows us to easily invoke the entrypoint from application components that
implement our new application model.
- the new trigger has a field that contains a `Configuration<CoreComponent>` —
in most cases, either `CoreComponent` will have to be updated with new trigger
and component configuration (not the case for our simple application model),
or an entirely new component can be defined and used in `Configuration<T>`.
- the trigger has a field that contains the Spin execution context — this is the
part of Spin that instantiates and helps execute the WebAssembly modules. When
creating the trigger (in the `new` function, you get access to the underlying
Wasmtime store, instance, and linker, which can be configured as necessary).

Finally, whenever there is a new event (in the case of our timer-based trigger
every `n` seconds), we execute the entrypoint of a selected component:

```rust
/// Execute the first component in the application configuration.
async fn handle(&self, msg: String) -> Result<()> {
    // create a new Wasmtime store and instance based on the first component's WebAssembly module.
    let (mut store, instance) =
        self.engine
            .prepare_component(&self.app.components[0].id, None, None, None, None)?;

    // spawn a new thread and call the `echo` function from the WebAssembly module 
    let res = spawn_blocking(move || -> Result<String> {
        // use the auto-generated WIT bindings to get the Wasm exports and call the `echo` export.
        let e = echo::Echo::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;
        Ok(e.echo(&mut store, &msg)?)
    }).await??;
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
- invoking the entrypoint `echo` is done in this example in a new Tokio thread —
this is an implementation choice based on the needs of the trigger.
- the return value from the component (a string in this example) can then be
used — in the case of the HTTP trigger, this is an HTTP response, which is then
returned to the client.

This is very similar to how the [HTTP](/http-trigger) and [Redis](/redis-trigger)
triggers are implemented, and it is the recommended way to extend Spin with your
own trigger and application model.

Embedding the new trigger in a Rust application is done by creating a new trigger
instance, then calling its `run` function:

```rust
let trigger = TimerTrigger::new(Duration::from_secs(1), app()).await?;
trigger.run().await
```

> We are exploring [APIs for embedding Spin from other programming languages](https://github.com/fermyon/spin/issues/197)
> such as Go or C#.
