title = "SIP xxx - The `spin test` command"
template = "main"
date = "2022-04-22T14:53:30Z"
---

Summary: A Spin command for testing Spin components locally.

Owner: radu.matei@fermyon.com

Created: April 22, 2022

Updated: April 22, 2022

## Background

Currently, testing a Spin component requires starting Spin by running `spin up`,
then creating an event that triggers the component. For example, for an HTTP
application, that involves sending an HTTP request, and for a Redis triggered
application, starting the Redis server and sending a new message on the
configured channel, which involves several CLI tools used in a specific
order before a component can be invoked.
When developing an application, executing the steps above with each code
iteration can be a repetitive task.

## Proposal

This document proposes adding a new `spin test` command that, given a `spin.toml`
manifest and a _test fixture_ file, executes a component's entrypoint using
the inputs supplied in the fixture, and returns an appropriate exit code in
case of success or failure.
This, in combination with the proposed `spin build` command, would significantly
simplify how Spin applications are worked on in the early phases of development.

This feature does not intend to replace unit unit tests, which are specific to
the language a component is implement in, nor does it intend to replace integration
tests — rather, it aims to provide a useful way of testing the entire execution
of a single component's Wasm module, but without requiring setting up and
invoking it through a specific trigger.

### Invoking components and test fixtures

We want to invoke and execute a component's entry point function with mock data,
without starting a trigger and waiting for external events. This is directly
related to the signature of the entry point function, which is specific for
each Spin trigger.

The proposed `spin build` command would use the data in the test fixture file
to populate the arguments for the entry point function. Let's consider the
function signature for an HTTP component:

```fsharp
// An HTTP request.
record request {
    method: method,
    uri: uri,
    headers: headers,
    params: params,
    body: option<body>,
}

// An HTTP response.
record response {
    status: http-status,
    headers: option<headers>,
    body: option<body>,
}

// The entrypoint for an HTTP handler.
handle-http-request: function(req: request) -> response
```

A "test fixture" is a JSON object that contains the following fields:

- input — structured input specific to a Spin trigger, that will be deserialized
and used as arguments for invoking the component's entry point. For HTTP
components, `input` would be the JSON representation of an HTTP request (as
defined by the WIT record).
- output — optional structured output specific to a Spin trigger, which is the
expected result of executing the component. For HTTP components, this would be
the JSON representation of an HTTP response (as defined by the WIT record), or
the expected result.

Byte array payloads are represented by their `base64` encodings.

A test fixture file contains an array of test fixtures.

## Future design considerations

- using test configuration (for environment variables, secrets, dependencies, or
host components)
