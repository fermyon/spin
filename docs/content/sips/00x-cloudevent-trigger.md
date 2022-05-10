title = "SIP xxx - CloudEvents trigger"
template = "main"
date = "2022-04-25T14:53:30Z"
---

Summary: A CloudEvents trigger for spin.

Owner: jiazho@microsoft.com

Created: April 24, 2022

Updated: April 24, 2022

## Background

Currently spin supports two triggers, one for Redis messages and one for HTTP requests. [CloudEvents](https://cloudevents.io/) are a new standard for eventing and received huge interests from the major cloud providers. Supporting CloudEvents could make spin a great solution for writing serverless applications. 


## Proposal

This document proposes adding features in the spin SDK to support CloudEvents. CloudEvents itself is a envelop for the underlying transport protocol, such as AMQP, Kafka, HTTP, etc. This proposal aims at providing a CloudEvent component for the [HTTP Protocol Bindings](https://github.com/cloudevents/spec/blob/main/cloudevents/bindings/http-protocol-binding.md). Here is an example shows the mapping of an event with an HTTP POST request in CloudEvent's binary format.
```
POST /someresource HTTP/1.1
Host: webhook.example.com
ce-specversion: 1.0
ce-type: com.example.someevent
ce-time: 2018-04-05T03:56:24Z
ce-id: 1234-1234-1234
ce-source: /mycontext/subcontext
    .... further attributes ...
Content-Type: application/json; charset=utf-8
Content-Length: nnnn
{
    ... application data ...
}
```

Creating an HTTP CloudEvents trigger is done by defining the top level application trigger in spin. The following code snippet shows the definition of a HTTP CloudEvents trigger.
```toml
# spin.toml
trigger = { type = "http", base = "/", schema = "cloudevents" }
```

The added `schema` attribute in trigger will tell spin application that it will expect the HTTP request and responses are CloudEvents.

> Note that the `schema` attribute is not the same as the `schema` attribute in the CloudEvents spec. The `schema` attribute in the spec is the schema of the payload.

We also allow users to define individual component to be CloudEvents components. For example, we could define a HTTP CloudEvents component in spin.toml:

```toml
# spin.toml
trigger = { type = "http", base = "/" }

[[component]]
id = "hello"
source = "target/wasm32-wasi/release/spinhelloworld.wasm"
description = "A simple component that returns hello."
schema = "cloudevents"
[component.trigger]
route = "/hello"
```

> Note that if there is no `schema` attribute in the application or individual component, the HTTP request and responses are not CloudEvents.

## The WebAssembly interface

The CloudEvent trigger is built on top of the
[WebAssembly component model](https://github.com/WebAssembly/component-model).
The current interface is defined using the
[WebAssembly Interface (WIT)](https://github.com/bytecodealliance/wit-bindgen/blob/main/WIT.md)
format, and is a function that takes the event payload as its only parameter:

```fsharp
// wit/ephemeral/spin-ce.wit

// The event payload.
record event {
    // The event type.
    type: string,
    // The event id.
    id: string,
    // The event source.
    source: string,
    // The event specversion.
    specversion: string,
    // The event data content type.
    datacontenttype: string,
    // The event data schema.
    dataschema: string,
    // The event subject.
    subject: string,
    // The event time.
    time: option<string>,
    // The event data.
    data: option<string>
}

// The entry point for a CloudEvent handler.
handle-cloudevent: function(event: event) -> expected<event, error>
```


This is the function that all CloudEvents components must implement, and which is
used by the Spin Redis executor when instantiating and invoking the component.

Notice that the function will return a `expected<event, error>` value. If the sink address is not set, the function will return `expected<event, error>` with `event` as the value. If the sink address is set, spin will make an outbound HTTP request to the sink address and return the response as the value.

Due to the [known issue](https://github.com/bytecodealliance/wit-bindgen/issues/171) of the cannonical ABI representation cannot exceeds the number of parameters (16) in the wasmtime API, the proposed WIT format cannot be compiled. I am proposing an alternative WIT format for CloudEvents:

```fsharp
// wit/ephemeral/spin-ce.wit

// The event payload.
type event = string

// The entry point for a CloudEvent handler.
handle-cloudevent: function(event: event) -> expected<event, error>
```

At the runtime, Spin will use CloudEvent SDK to parse the event payload and invoke the function. For example, take a look at the [CloudEvents SDK Rust](https://github.com/cloudevents/sdk-rust) library.

## The CloudEvents Spin SDK
```rust
// A Spin CloudEvents component written in Rust
use anyhow::Result;
use spin_sdk::{
    event::{Event},
    event_component,
};

/// A simple Spin event component.
#[cloud_event_component]
fn trigger(event: Event) -> Result<Event, _> {
    println!("event is {}", event.id());
    Ok(event)
}
```

```go
// A Spin CloudEvents component written in Go
package main

import (
 "fmt"
 "context"

 spin "github.com/fermyon/spin/sdk/go/event"
)

func main() {
 spin.ReceiveEvent(func(event spin.Event) {
  fmt.Printf("%s", event)
  spin.SendEvent(ctx, event)
 })
}
```

## Future design considerations

#### More transport protocols bindings
- Kafka binding
- AMQP binding
- NATS binding

#### Filtering based on event attributes
```toml
[[component]]
id = "filter"
source = "target/wasm32-wasi/release/spinhelloworld.wasm"
description = "A simple component that filters events based on event attributes."
schema = "cloudevents"
[component.trigger]
route = "/filter"
[component.filter]
ce.type = "com.example.someevent"
ce.source = "/mycontext/subcontext"
```

#### Generic CloudEvent component
A generic CloudEvents component is defined in the following way:
```rust
// A Spin CloudEvents component written in Rust
use anyhow::Result;
use spin_sdk::{
    event::{Event},
    cloud_event_component,
};

/// A simple Spin event component.
#[cloud_event_component]
fn trigger(event: Event) -> Result<Event, _> {
    println!("event is {}", event.id());
    // do something with the event
    Ok(event)
}
```

It is trigger-agnostic, at least within the supported CloudEvents protocol bindings. You can see the list of supported protocols in the [CloudEvents documentation](https://github.com/cloudevents/spec/blob/main/cloudevents/bindings). The benefits of doing this are:
1. Rapid prototyping: you can quickly prototype your event components and test them locally using HTTP bindings. Once you are confident that they are working, You can switch the trigger to a different type, without having to modify the code.
2. Reusability: you can reuse the same event components with different protocols bindings, such as AMQP and Kafka.
3. Chaining: you can chain multiple event components together since they share the same component signature.

#### CloudEvents trigger

Creating an CloudEvents trigger is done when [configuring the application](/configuration)
by defining the top-level application trigger:

```toml
# spin.toml
trigger = { type = "cloudevent" }
```

Then, when defining the component (in `spin.toml`), you can set the protocol binding for the component. For example:

- an HTTP CloudEvents component:

```toml
[component.trigger]
binding = "http"
```

- an Kafka CloudEvents component (optional):

```toml
[component.trigger]
binding = "kafka"
broker = ["localhost:9092", "localhost:9093"]
topic = "mytopic"
group = "mygroup"
```

- an AMQP CloudEvents component (optional WIP):

```toml
[component.trigger]
binding = "amqp"
broker = "localhost:5672"
exchange = "myexchange"
routing_key = "myroutingkey"
```

You can also set the sink address for the component that returns a CloudEvent. For example:

```toml
[component.trigger]
binding = "http"
sink = "http://localhost:8080/someresource"
```

Note that the sink address is only used when the component is invoked. The component will make a outbound HTTP request that includes the CloudEvents to the sink address.