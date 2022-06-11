title = "SIP xxx - Logs and Metadata WASI filesystem"
template = "main"
date = "2022-06-07T22:49:37"

---

Summary: Expose the logs and metadata to components as a filesystem

Owner: me@mitchellhynes.com

Created: June 7th, 2022

Updated: June 8th, 2022

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

If Spin exposed a "monitoring filesystem" that was accessible by WASI FS we
could open the door to many hypervisor-like tools.

### The `~/.spin/` directory

The `~/.spin/` directory is where Spin keeps logs and other ephemeral data for
apps.

```
tree ~/.spin/
└── appname
    ├── key-value
    └── logs
        ├── componentname_stderr.txt
        └── componentname_stdout.txt
```

## Proposal

WASI provides a system interface where components can access files as if they
were system files in their program, except those files don't have to be on disk
and don't even have to be files at all.

### `~/.spin/`: Escape 2 Madigascar

Exposing `~/.spin/` to components doesn't work because all files and folders
remain in the same state they were mounted at. Any new logs won't be included
when read again.

If instead of mounting a `~/.spin/` directory on the host machine, we allowed
users to mount a directory internal to Spin in-memory this would be updated
on subsequent reads while having the access control of a mounted directory.

#### Mount: `/.spin/monitor/`

Inside each app folder are the stdout/stderr for each component. This is what we
want to make available as a file system for guest modules. To do so they need to
be explicitly mounted. However since we don't want to mount the actual
file system behind Spin because it's ephemeral and temporary, we should be
explicit about how this file system is different from the host file system.

> **_Sidenote:_** `spin_monitoring` stops conflicts between WASI FS and some
> directories that might be mounted to the host's file system.

> `spin_monitoring` also enables other paths like `/.spin/config/`

```diff
-files = [{ source = "~/.spin/[...]/logs", destination = "/logs" }]
+files = [{ source = ".spin/monitor/", destination = "/monitor/logs" }]
+spin_monitoring = true
```

The `/.spin` path is a special path in Spin where logs are accessed from. If
users provide this path to their components under the source key then the
component's logs would be accessible to the guest module.

```sh
tree /.spin/monitor/
└── logs
    └── [componentname]
        ├── stderr.txt
        └── stdout.txt
```

#### Mount: `/.spin/app/manifest{.json&.toml}`

Aside from the logs, we could also expose the manifest of the current running
app as a filesystem too.

```sh
tree /.spin/config/
└── [appname]
    ├── manifest.json
    └── manifest.toml
```

#### Persisting to disk

Some users will will probably rightfully expect the `~/.spin/` directory to
match the structure of what they are given in Spin. In the case that a user
needs to read some logs, the Spin directory should be familiar to them if
they've interacted with the Spin mount source interface before.

_On the host:_

```sh
tree ~/spin/
└── [appname]
    └── logs
        └── [componentname]
            ├── stderr.txt
            └── stdout.txt
```

## Conclusion

With more of Spin's state exposed to components users can expose more
sophisticated tools. If we redesigned the Spin directory to be a simple,
persisted version of a file system exposed to components we could consolidate
the two into one idiomatic interface.
