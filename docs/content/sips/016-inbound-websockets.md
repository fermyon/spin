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
- Document consistency guarantees in Spin's SQLite support so developers can rely on them for application state management

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
  // - `token`: a globally unique identifier which may be used to
  //   send frames to this WebSocket using
  //   `outgoing-websocket#send-frame`. Note that any application
  //   may use this `token` to send frames to the WebSocket for as
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
  handle-new: func(token: string, request: incoming-request, response-out: response-outparam);

  // Handle the most recent frames received for the specified
  // WebSocket.
  handle-frames: func(token: string, frames: list<received-frame>);
}

interface inbound-websocket-send {
  use websocket-types.{frame, future-send-frame};

  // Attempt to send the specified frame to specified WebSocket.
  //
  // This will fail if the WebSocket is no longer open. Otherwise,
  // if the frame is an instance of `close`, the connection will
  // be closed after sending the frame.
  send-frame: func(token: string, frame: frame) -> future-send-frame;
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

Each open WebSocket will be assigned a unique, opaque token (e.g. a 128-bit, base-64-encoded, securely-generated random number, or perhaps a signed, encrypted auth token) which may be used by any component of any type (e.g. `http`, `redis`, `websocket`, etc.) to send frames to the client via `fermyon:spin/inbound-websocket-send#send-frame`. For example, a chat application might use these tokens (or aliases thereof) to route chat messages within a group.

## Scalability and Reliability

The design described in this proposal is intended to scale horizontally in a distributed cloud environment. For example, a cluster of [websocket-bridge](https://github.com/fermyon/websocket-bridge) nodes could accept inbound WebSocket connections and dispatch incoming frames to separate cluster of Spin nodes, spreading the load evenly across the latter such that each frame from a given connection potentially handled by a different node.

Spin (and any associated infrastructure, such as `websocket-bridge`, load balancers, etc.) will deliver frames to and from WebSocket clients on a best-effort basis, guaranteeing in-order delivery with no gaps or repetitions, and conservatively closing connections whenever that cannot be achieved (e.g. due to network partitions, server failures, etc.). Applications are therefore responsible for recovering from unexpected connection closures if required. For example, the client side of a collaborative editing application could reconnect and re-synchronize with the server using a [CRDT](https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type) protocol, only alerting the user if that process takes more than a moment.

## State Management

Many applications will need to maintain state that lives for at least the lifetime of a given WebSocket connection, and many will also need to share state across multiple connections. In a traditional, monolithic, long-lived server application, such state is usually stored in memory, with access to shared state managed using e.g. mutexes. In this proposal, however, each frame may be received by a different instance, possibly running on a different node in a cluster. Therefore, any state must be kept in a persistent store outside of a given instance's memory, and the behavior of an application is highly sensitive to the consistency model of the persistent store(s) it uses.

For example, consider the following scenario: two users, Alice and Bob, enter a chat room at nearly the same time. The chat application is responsible for promptly notifying both users that the other has arrived. One way it might do that is: for each new connection, add the user to the requested room via a write to a database, then query the database to discover any other users present in the room.

If the database provides global, strict serlializability, this will always work correctly: if the app adds Alice first and doesn't see Bob, then later when Bob arrives the app will see Alice, send her a notification, and report to Bob that Alice is already there. Regardless of which order they arrive, they'll both discover each other as soon as the last one arrives.

However, if the app uses a database that's distributed and eventually consistent, Alice and Bob may never see each other; they may end up writing to and querying different database nodes and miss each other due to state synchronization delays.  In this case, the app has a few options:

1. Switch to a state store that offers global, strict-serlializability where it's needed.
2. Switch to an eventually consistent store that provides asynchronous notifications of state changes, and propagate those notifications to clients as appropriate.
3. Stick with the original eventually consistent store, but poll it periodically to discover new arrivals.

The first two of those options are appealing from a both a developer and user experience perspective. The third one requires the app developer to confront a fundamental trade-off: polling too often creates unreasonable load on the database, application server, etc., while polling too infrequently results in a poor user experience. Indeed, one of the big reasons to provide WebSocket support is to avoid polling and its trade-offs.

Unfortunately, as of this writing, neither of Spin's built-in persistent stores (SQLite and key-value) provide any sort of explicit consistency guarantees, so apps must either provide their own store (e.g. a PostgreSQL, MySQL, or Redis server) or use the third option.

Therefore, we propose to officially document and support a consistency model for Spin's SQLite implementation which provides (optional) strict serializability (i.e. both [linearizability and serializability](http://www.bailis.org/blog/linearizability-versus-serializability/)). Specifically:

- All writes (e.g. `INSERT`s, `UPDATE`s, and `DELETE`s) are linearized (i.e. each database connection will see writes applied in the same order)
- A `SELECT` query following a write on the same database connection is guaranteed to see that write and all previous writes
- A `SELECT` query in a [BEGIN IMMEDIATE](https://sqlite.org/lang_transaction.html) transaction is guaranteed to see all the writes completed up to that point in time, regardless of whether it is preceded by a write on the same database connection
- In contrast, a `SELECT` query which neither follows a write on the same connection nor is part of a `BEGIN IMMEDIATE` transaction may see old data. For example, if an app opens a database connection, performs a write, closes the connection, then opens a new connection and does a `SELECT`, it might not see the write performed by the original connection.

A consequence of the last point above is that if `fermyon:spin/inbound-websocket-receive#handle-frames` is invoked twice conscutively for the same connection, with each invocation delivering a batch of frames in order, and the first invocation writes to the database, the second invocation might not see the result of that write. It must either perform its own write prior to querying or else do the query inside a `BEGIN IMMEDIATE` transaction.

## Future Possibilities

- [WebTransport](https://www.w3.org/TR/webtransport/) is a new, more advanced protocol for high-performance client-server networking based on HTTP/3. It's not yet supported by all popular browsers, but could be worth supporting in Spin eventually.
- Outbound WebSockets could be supported similarly to this proposal, with short-lived instantiations created as frames arrive from the remote server. However, server-to-server WebSockets are rare, so it's not clear how useful this would be.
