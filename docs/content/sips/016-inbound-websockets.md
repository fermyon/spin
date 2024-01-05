title = "SIP 016 - Inbound WebSocket Support"
template = "main"
date = "2024-01-05T23:00:00Z"

---

Summary: Add support for handling inbound WebSocket connections in Spin.

Owner(s): joel.dice@fermyon.com

Created: Jan 05, 2024

## Background

WebSocket is a protocol designed for framed, simultaneous, two-way communication over TCP, usually between a client application such as a web browser and an HTTP server. It is often used by web applications to provide real-time state updates (e.g. a dynamically-updating event counter or log reporter) and interactivity (e.g. real-time chat and multiplayer games). Other HTTP-based options include "long polling" and server-sent events (SSE), neither of which fit the Spin model very well given that a naive implementation requires that component instances run continuously or nearly continuously.

Support for inbound WebSocket connections in Spin is something we've wanted to do for a while, but until now we did not have a clear vision of how this would fit the "short-lived, stateless instance" model that is central to how Spin works.

Note that support for outbound WebSocket connections could also be useful, but is not in scope for this SIP.

## Proposal

- Define WIT interfaces for sending and receiving WebSocket frames (see below)
    - These interfaced are deliberately designed to avoid the need for long-lived component instances
    - Eventually, we may want to propose this as a WASI standard, but to begin with we'll consider it experimental and Spin-specific
- Add host trigger and component support to Spin for said interfaces, using e.g. `hyper` and `tokio-tungstenite`

### WIT Interfaces and World

```
package fermyon:spin;

interface websocket-types {
  use wasi:io/poll.{pollable};
  
  variant error-code {
    // Indicates the connection to the client was lost prior to or
    // while attempting to send a frame.
    connection-lost
  }
  
  // From https://datatracker.ietf.org/doc/html/rfc6455#section-7.4.1
  variant close-code {
    normal,
    protocol,
    unsupported,
    status,
    abnormal,
    invalid,
    policy,
    size,
    extension,
    error,
    restart,
    again
  }

  // From https://datatracker.ietf.org/doc/html/rfc6455#section-5.2
  record close-frame {
    code: close-code,
    reason: option<string>
  }

  // From https://datatracker.ietf.org/doc/html/rfc6455#section-5.2
  variant frame {
    text(string),
    binary(list<u8>),
    ping(list<u8>),
    pong(list<u8>),
    close(option<close-frame>)
  }
  
  // Represents a frame received from the client
  record received-frame {
    // Sequence number assigned to this frame
    //
    // Each frame received from the client is assigned a sequence
    // number that determines its order with respect to other frames.
    // This may be useful when logging frames in an
    // eventually-consistent data store, for example.
    sequence-number: u64,
    
    // The content of the frame
    frame: frame
  }
  
  // Represents a future representing the completion or failure of
  // sending a frame to a client.
  resource future-send-frame {
    // Returns a pollable which becomes ready when either the frame
    // has been sent, or an error has occurred. When this pollable is
    // ready, the `get` method will return `some`.
    subscribe: func() -> pollable;
    
    // Returns the result of sending a frame to a client, once it
    // has either completed successfully or errored.
    //
    // The outer `option` represents future readiness. Users can wait
    // on this `option` to become `some` using the `subscribe` method.
    //
    // The outer `result` is used to retrieve the response or error
    // at most once. It will be success on the first call in which the
    // outer option is `some`, and error on subsequent calls.
    //
    // The inner `result` represents that either the frame was sent
    // successfully, or that an error occurred.
    //
    // Note that a success result from this function does *not*
    // guarantee that the client received the frame (or that it ever
    // will). The frame will be sent asynchronously and might not
    // be delivered due to an unexpected close event or network
    // failure. Applications which require delivery confirmation
    // must handle that themselves.
    get: func() -> option<result<result<_, error-code>>>;
}

interface inbound-websocket-receive {
  use wasi:http/types.{incoming-request, response-outparam};
  use websocket-types.{frame, close-frame};

  // Construct a new handler for the specified request, which the
  // client has requested to be upgraded to a WebSocket.
  //
  // - `id`: a globally unique identifier which may be used to send
  //   frames to this WebSocket using
  //   `outgoing-websocket#send-frame`. Note that any application
  //   may use this `id` to send frames to the WebSocket for as
  //   long as it is connected, which means it should be treated as
  //   a secret and shared only with trusted parties.
  //
  // - `request`: a resource providing the request's method, path,
  //   headers, etc.
  //
  // - `response-out`: a resource which may be used to send a
  //   response to the client. If (and only if) the response has a
  //   status code of 101, then the connection will be upgraded to a
  //   WebSocket.
  handle-new: func(id: string, request: incoming-request, response-out: response-outparam);

  // Handle the most recent frames received for the specified
  // WebSockets. `frames` consists of a mapping from WebSocket ids
  // to ordered lists of frames.
  handle-frames: func(frames: list<tuple<string, list<received-frame>>);
}

interface inbound-websocket-send {
  use websocket-types.{frame, future-send-frame};

  // Attempt to send the specified frame to specified WebSocket.
  //
  // This will fail if the WebSocket is no longer open. Otherwise,
  // if the frame is an instance of `close`, the connection will
  // be closed after sending the frame.
  send-frame: func(id: string, frame: frame) -> future-send-frame;
}

world inbound-websocket-handler {
  // Incoming requests which do not request a WebSocket upgrade
  // will be passed to `wasi:http/incoming-handler#handle`
  include wasi:http/proxy;

  // Incoming requests which request a WebSocket upgrade will be
  // passed to `incoming-websocket-receive#handle-new`.
  export inbound-websocket-receive;

  import inbound-websocket-send;
}
```

### Spin Implementation

We’ll add a new trigger type called `websocket` which will be handled by the `spin-trigger-http` crate. This crate will handle normal (non-WebSocket) incoming requests as usual: instantiating the component, calling the `wasi:http/incoming-handler#handle` method, and discarding the instantiation when the entire response has been sent. For WebSocket requests (i.e. those with headers requesting a connection upgrade per the WebSocket protocol), it will instead call `fermyon:spin/inbound-websocket-receive#handle-new`, and if that produces a valid 101 ("Switching Protocols") response, the host component will discard the instance and upgrade the connection to a WebSocket.

As frames arrive from the client, Spin will create new instances and pass the frames to `fermyon:spin/inbound-websocket-receive#handle-frames`, attempting to batch frames into a minimum number of instantiations without adding delay. If an instance fails unexpectedly (e.g. due to a trap), Spin will close the connection as a conservative measure given that the app state for that connection may no longer be consistent. Clients may attempt to reconnect in this case, if appropriate.

If a connection is lost unexpectedly prior to receiving a WebSocket `close` frame from the client, Spin will attempt synthesize such a frame and deliver to the application, giving it an opportunity to clean up associated state. Note, however, there is no guarantee that the app will always receive a `close` frame promptly or at all -- external factors such as network failures or power loss might delay or prevent that, so apps should not rely exclusively on it.

Each open WebSocket will be assigned a unique, opaque ID (e.g. a 128-bit, base-64-encoded, securely-generated random number) which may be used by any component of any type (e.g. `http`, `redis`, or `websocket`) to send frames to the client via `fermyon:spin/inbound-websocket-send#send-frame`. For example, a chat application might use these IDs (or aliases thereof) to route chat messages within a group.

## Scalability and Reliability

The design described in this proposal is intended to scale horizontally in a distributed cloud environment. For example, a cluster of [websocket-bridge](https://github.com/fermyon/websocket-bridge) nodes could accept inbound WebSocket connections and dispatch incoming frames to separate cluster of Spin nodes, spreading the load evenly across the latter such that each frame from a given connection potentially handled by a different node.

Spin (and any associated infrastructure, such as `websocket-bridge`, load balancers, etc.) will deliver frames to and from WebSocket clients on a best-effort basis, guaranteeing in-order delivery with no gaps or repetitions, and conservatively closing connections whenever that cannot be achieved (e.g. due to network partitions, server failures, etc.). Applications are therefore responsible for recovering from unexpected connection closures if required. For example, the client side of a collaborative editing application could reconnect and re-synchronize with the server using a [CRDT](https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type) protocol, only alerting the user if that process takes more than a moment.

## Future Possibilities

- [WebTransport](https://www.w3.org/TR/webtransport/) is a new, more advanced protocol for high-performance client-server networking based on HTTP/3. It's not yet supported by all popular browsers, but could be worth supporting in Spin eventually.
- Outbound WebSockets could be supported similarly to this proposal, with short-lived instantiations created as frames arrive from the remote server. However, server-to-server WebSockets are rare, so it's not clear how useful this would be.
