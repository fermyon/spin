title = "SIP xxx - The Admin API
template = "main"
date = "2022-06-07T22:49:37"

---

Summary: An Administration API for Spin.

Owner: me@mitchellhynes.com

Created: June 7th, 2022

Updated: June 7th, 2022

## Background

Apart from reading the file system of the host machine, there's no way to read
logs or manifest of a running Spin app. This means users are limited in what
they know about a Spin app while it's running, unless they have access to the
environment it is on.

[Laundromat](https://github.com/ecumene/laundromat) is a tool for reading logs
and organizing them in a quicker format for developer velocity, and it gets
around this by serving the logs of a Spin app as a REST API. This is limited
because

- it must have access to the file system which users may not have access to
- being a server itself it must be forwarded out of whatever network users are
  running it in, along with Spin itself
- the file system is itself limited as new logs won't notify the user.
  system file events could be used but they're notoriously platform dependent.

If Spin exposed an "administrator's API" that is handled by the same route that
Spin apps are served on we could open the door to many hypervisor-like tools.

## Proposal

This SIP proposes a new API for the server to provide access to the logs and
manifest of a Spin app, should you provide the `--admin` flag. The `--admin`
flag exposes the `/_spin/v1/` API, which can be used to both introspect the logs
of components and the manifest of the app itself. The home of the API would be
configured with a separate flag called `--admin-alias <API_PREFIX>`.

**spin up**

```
    -a, --admin
            Expose logs and manifest as an admin API

        --admin-alias <API_PREFIX>
            Admin API home (default is /_spin/)
```

### API

#### `/<API_PREFIX>/v1/logs`

This provides a service for introspecting the logs of a running Spin app. It
could be polled for new logs, and in the future it could be used to stream logs.

- `GET /logs` - Returns all logs of the app.
- `GET /logs?stderr` - Filter for only standard error logs
- `GET /logs?stdout` - Filter for only standard out logs
- `GET /logs/<id>` - Returns the logs of the component with the given id.
- `GET /logs/<id>?stderr` - Filter for a component's standard error logs
- `GET /logs/<id>?stdout` - Filter for a component's standard out logs

**Notes:**

GET implies an HTTP transaction. Where the contents of the log files are read
send to the requester. Spin doesn't buffer logs from components so reading from
the file system will have to do for now.

`<id>` is the component ID, URL serialized if need be.

#### `/<API_PREFIX>/v1/manifest`

This provides a service for introspecting the manifest of a running Spin app.

- `GET /manifest` - Returns the manifest of the app in TOML format.
- `GET /manifest?format=toml` - Returns the manifest of the app in TOML format.
- `GET /manifest?format=json` - Returns the manifest of the app in JSON format.

## Future design considerations

- should the logs be optionally streamed?
- will reading the logs from file cause issues? Should the logs be buffered
  instead?
- would an `admin-gui` SIP be an interest to others? This would be an
  integration of [Laundromat](https://github.com/ecumene/laundromat) **in Spin**.
  Or, something like it.
- should this be an authenticated API for use in production environments?
