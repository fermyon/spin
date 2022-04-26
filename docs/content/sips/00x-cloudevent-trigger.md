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

This document proposes adding a new trigger for CloudEvents. The triggers are invoked by a CloudEvent source. The CloudEvent source is a event provider service that sends CloudEvents to spin, such as Kafka topics, HTTP requests, AMQP messages. For example, the [CloudEvents spec](https://github.com/cloudevents/spec/tree/main/cloudevents/bindings) list a few protocol bindings including AMQP, HTTP, Kafka etc.

This proposal aims at providing a CloudEvent component for the [HTTP Protocol Bindings](https://github.com/cloudevents/spec/blob/main/cloudevents/bindings/http-protocol-binding.md). This example shows the mapping of an event with an HTTP POST request.
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


## Future design considerations

- CloudEvents bindings for different protocols, such as AMQP, Kafka, etc.
- Filter events based on event attributes.