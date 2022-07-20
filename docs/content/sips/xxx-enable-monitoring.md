title = "SIP xxx - Flag to enable access to the app's Spin home directory"
template = "main"
date = "2022-06-07T22:49:37"

---

Summary: Open application's Spin home directory as a filesystem to components

Owner: me@mitchellhynes.com

Created: June 7th, 2022

Updated: June 20th, 2022

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
└── [appname]
    └── logs
        ├── [componentname]_stderr.txt
        └── [componentname]_stdout.txt
```

## Proposal

WASI provides a system interface where modules can access files in the host
system by pre-opening file descriptors.

Mounting `~/.spin/` to components doesn't work because all files and folders
remain in the same state they were mounted at. Any new logs won't be included
when read again.

Instead of mounting the entire Spin home directory, we could mount only the
application's directory within that guest file system (`/.spin/`). This would
allow users to view logs of their application if a flag was enabled.

#### Mount: `/.spin/`

Inside each app folder are the stdout/stderr for each component. This is what we
want to make available as a file system for guest modules. To do so the user
simply enables a flag to expose the Spin home directory for their app.

```diff
+enable_monitoring = true
```

The `/.spin` path is a special path in Spin where logs and other state data are
accessed from. This is a 1-1 map with the `~/.spin/[appname]` directory on the
host machine. Note that not all apps are accessible from this directory.

```sh
tree /.spin/
└── logs
    └── [componentname]
        ├── stderr.txt
        └── stdout.txt
```

## Conclusion

With more of Spin's state exposed to components, users can create more
sophisticated apps and monitoring tools.
