title = "SIP XXX - Spin Wasm HTTP URLs"
template = "main"
date = "2022-06-29T02:55:31Z"
---

Summary: A standard way of defining an HTTP(S) URL to a Wasm module

Owner: matt.butcher@fermyon.com

Created: June 29, 2022

## Background

Currently, when a WebAssembly component is loaded, it can only be loaded from a local file source.

For example, to load `scoreboard.wasm`, the current `spin.toml` specifies `source` as a path:

```toml
[[component]]
id = "scoreboard"
source = "components/scoreboard.wasm"
environment = {REDIS_ADDRESS = "redis://localhost:6379/"}
[component.trigger]
route = "/score"
```

In this case, any WebAssembly module to be used by Spin must be present on the filesystem prior to Spin starting.
Moreover, there is no indication of what the version of said object is,
nor any way to ensure that the file located at that path is the one intended by the developer.

Furthermore, it is currently not possible to distribute WebAssembly components intended for reuse in Spin applications.
For example, the Finicky Whiskers site must have a static copy of the `fileserver.wasm` module in its repository
because there is no method by which it can instruct tooling to fetch the `fileserver.wasm` from another source.

While the solution here is not intended to take the place of a package manager,
it is intended to provide a convenient way of referencing a single remote Wasm component
that can be fetched over HTTP/HTTPS or over the `file` scheme.
Doing this immediately enables re-use for utility components like Bartholomew, file server, and so on.

## Proposal

This SIP proposes that Wasm components in Spin be loadable by HTTP/HTTPS URL in addition to path.
Furthermore, it specifies a method by which a cryptograph hash be appended to a URL for verification.

This specification proposes an HTTP/HTTPS format,
and also suggests how the `file` protocol may be augmented to provide hash information.
This specification does not define any other protocol formats,
leaving open the possibility that future SIPs may.

### URL Format for HTTP/HTTPS

A URL to a Spin resource is a standard HTTP or HTTPS URL that points to a location that delivers a format Spin can load.
For example, a URL to a Wasm module on GitHub may look like this:

```
https://github.com/fermyon/bartholomew/releases/download/v0.3.0/bartholomew.wasm
```

No requirements should be made about the structure of the URL, other than that it is valid.

Including the above in a `spin.toml`'s `source` field looks like this:

```toml
[[component]]
source = "https://github.com/fermyon/bartholomew/releases/download/v0.3.0/bartholomew.wasm"
```

Optionally, a URL as presented in a `spin.toml` or similar file may have a hash appended using an anchor.
Implementations MUST support SHA256 and SHOULD support SHA512.

The format of the hash in the anchor is `$ALGO:$HASH_BYTES`

```toml
[[component]]
source = "https://github.com/fermyon/bartholomew/releases/download/v0.3.0/bartholomew.wasm#sha256:723ade692c4f8b047b6e90b7c9625a57c9819e0f1bd29bd75eae5b79dd436c4b"
```

The anchor (`#...`) is used as local information only, and MUST be stripped before it is sent to the remote server.
Consequently, an application that loads the above source would access the following remote URL: https://github.com/fermyon/bartholomew/releases/download/v0.3.0/bartholomew.wasm`.

An implementation SHOULD proceed according to the following steps when loading a Wasm HTTP/HTTPS URL:

1. Read the string
1. Parse or validate the URL
    * If the parse fails, implementation MAY treat the value as a local file path
1. (OPTIONAL) Check the local cache for an existing copy of this URL's data
1. Remove the anchor (`#...`) if present, storing the anchor text for later
1. Issue an HTTP GET request to fetch the resource
    * If the HTTP status is not 200, error with a message
    * If the Content-Type of the data is `application/x-octet-stream`, `application/octet-stream`, or `application/wasm`, the content SHOULD be treated as WebAssembly
    * Behavior for any other Content-Type is undefined
1. (OPTIONAL) Follow redirects
1. Download the result
    * If the download fails, display an error
1. If an anchor was present and the algo for the anchor is supported (e.g. SHA256), verify the result against the anchor's hash
    * If the check fails, abort with an error and destroy the fetched data
    * If the algo is not supported, abort with an error and destroy the fetched data
1. (OPTIONAL) Cache the fetched object's data in local storage


From here, loading of the module proceeds as normal.

### URL format for Wasm FILE protocol

A `source` may also use the `file` scheme to identify a source.
The `file` scheme format may also use an anchor to store the cryptographic hash data.

```toml
source = "file:///usr/local/bartholomew.wasm#sha256:723ade692c4f8b047b6e90b7c9625a57c9819e0f1bd29bd75eae5b79dd436c4b"
```

The implementation SHOULD proceed as follows:
1. Read the string
1. Parse or validate the URL
    * If the parse fails, implementation MAY treat the value as a local file path
1. (OPTIONAL) Check the local cache for an existing copy of this URL's data
1. Remove the anchor (`#...`) if present, storing the anchor text for later
1. Resole and load the data from the local filesystem
    * If an error occurs during file I/O, abort with an error
1. If an anchor was present and the algo for the anchor is supported (e.g. SHA256), verify the result against the anchor's hash
    * If the check fails, abort with an error and destroy the fetched data
    * If the algo is not supported, abort with an error and destroy the fetched data
1. (OPTIONAL) Cache the fetched object's data in local storage


### Manifest version

While the format of the `spin.toml` does not change, the semantics of file paths does.
For that reason, the manifest version may be incremented.

## Backward Compatibility

Older versions of Spin will simply fail to find the `source` if a URL is used instead of a path.

## Design Options

### Local Cache

As specified in the flows above, the design is compatible with a local cache that may store the binary data keyed by the complete URL of the `source`.
Such a cache would reduce the amount of network traffic,
reduce redundant fetches,
and reduce redundant verifications.

If a URL contains a hash, its data is by definition immutable.

If a URL does not contain a hash, the data MAY be treated as immutable.
Alternately, implementations may determine that the absence of a hash implies mutability,
and apply an appropriate caching strategy.

### No Support for `file`

The `file` scheme is not necessary to support

### Use Dedicated Hash Field

Instead of using an anchor, it is possible to alter the `spin.toml` to have a designed field for storing hashes.

```toml
[[component]]
source = "https://github.com/fermyon/bartholomew/releases/download/v0.3.0/bartholomew.wasm"
source_hash = "sha256:723ade692c4f8b047b6e90b7c9625a57c9819e0f1bd29bd75eae5b79dd436c4b"
```

This would introduce a breaking change to the `spin.toml`,
but has the advantage of not applying a novel semantic to URL anchors.
