title = "SIP 008 - Distributing Spin applications using OCI registries"
template = "main"
date = "2023-01-04T01:01:01Z"

---

Summary: This improvement proposal describes the reasoning and implementation
for distributing Spin applications using OCI registries.

Owners: radu@fermyon.com

Created: January 4, 2023

## Background

Since its first release, Spin has used
[Bindle](https://github.com/deislabs/bindle) as the mechanism for distributing
applications — an experimental aggregate object storage project originally
designed to distribute WebAssembly applications and their supporting files.

Since Bindle is a relatively early project, using it has surfaced a few issues
that could cause us to reconsider using it as the primary mechanism of
distribution:

* because there are no managed Bindle services, distributing Spin applications
    requires users to run and manage their own infrastructure, increasing the
    level of complexity required for the most simple applications
* when managing the Bindle infrastructure, scaling the infrastructure is an
    [unsolved issue for the Bindle project](https://github.com/deislabs/bindle/issues/263)

[OCI](https://opencontainers.org), or the Open Container Initiative, emerged as
the standard for packaging and distributing container images. Initially used for
container images, the use of container registries has been expanding to more
artifact types with the introduction of the
[OCI Artifacts](https://github.com/opencontainers/artifacts) project.

All major cloud providers offer managed registry services, and among them, there
are services that already support distributing other artifact types.

## Proposal

This SIP proposes that Spin should support distributing applications using OCI
registries. This would solve the two issues outlined above:

* because of the plenty managed OCI registry services, users could use
    existing services — such as GitHub Container Registry, Docker Hub, AWS Elastic
    Container Registry, Azure Container Registry, Google Artifact registry, or
    others.
* if using a managed service, scaling the infrastructure is no longer a concern
    for users; if users decide to self-host, horizontally scaling a container
    registry should be a more straightforward task, with more resources and
    projects whose goal is scaling available when compared to Bindle.

### The user experience

The implementation should give users the ability to push an application to a
compatible registry, pull an application locally, and run an application:

```bash
$ spin oci push ghcr.io/<username>/my-spin-application:v1
INFO spin_publish::oci::client: Pushed "https://ghcr.io/v2/<username>/my-spin-application/manifests/sha256:9f4e7eebb27c0174fe6654ef5e0f908f1edc8f625324a9f49967ccde44a6516b"

$ spin oci pull ghcr.io/<username>/my-spin-application:v1
INFO spin_publish::oci::client: Pulled ghcr.io/<username>/my-spin-application:v1@sha256:9f4e7eebb27c0174fe6654ef5e0f908f1edc8f625324a9f49967ccde44a6516b

$ spin up --oci ghcr.io/<username>/my-spin-application:v1
INFO spin_publish::oci::client: Pulled ghcr.io/<username>/my-spin-application:v1@sha256:9f4e7eebb27c0174fe6654ef5e0f908f1edc8f625324a9f49967ccde44a6516b
Serving http://127.0.0.1:3000
```

The commands and arguments shown above are not final.

### Authentication

Historically, the `docker` CLI has been the toolchain used to interact with
container images and registries — as a result, given its popularity, the
`spin oci` functionality should be able to re-use credentials for already
logged-in users. Additionally, most container registry services have
instructions on how to log in to their services using the `docker login`
command.

However, having Docker installed locally should not be a prerequisite for using
Spin. To address this, a `spin oci login` command should be implemented that
authenticates the Spin CLI to the desired registry instance:

```bash
$ spin oci login --username <username> --password <password>
# OR
$ echo $CONTAINER_REGISTRY_PASSWORD | spin oci login --username <username> --password-stdin
```

This user experience would mirror
[the `docker login` command](https://docs.docker.com/engine/reference/commandline/login/).

### Migrating applications from Bindle to an OCI registry

Changing the distribution mechanism for Spin applications is a breaking change
for the project — to address this, the project should provide functionality
that users can use to migrate their applications from Bindle to an OCI registry.

To take the ephemeral usefulness of this tool into account, and to prevent
future breaking changes by the needing to remove it,
this functionality would be best suited as a Spin plugin, installed
when needed and distributed separately:

```bash
spin bindle2oci \
    --bindle-server <server> \
    --bindle-username <username> \
    --bindle-password <password> \
    ---bindle <name> \
    --oci <new-reference>
```

### Implementation

A Spin application is made up of metadata and component information
together with the Wasm modules and static assets that made up those components.
So conceptually, a Spin application is not *a single artifact*, but rather
multiple distinct objects.

This SIP proposes that, when distributed using an OCI registry, a Spin
application would become a new OCI *artifact* with multiple layers
(making the distinction clear, as *images* and *artifacts* are separate
entities in OCI). Specifically, the media type used in this implementation
for a Spin application is `application/vnd.fermyon.spin.application.v1+config`,
and each Wasm module and static asset from the Spin components becomes an individual
*layer* in the resulting registry entity. Because each file and Wasm module becomes
a separate *layer*, we can efficiently de-duplicate and distribute applications.

The remaining question is representing the Spin application definition — Spin
introduced the internal representation of a *locked application* - an
intermediate representation of a Spin application that *can* have a way to
content-address Wasm modules and static assets. The implementation for this SIP
uses the Spin locked application as the OCI configuration object.

Below is an example of a Spin application with one component and one static
asset, its OCI manifest, locked application, and resulting local directory
structure when pulling the application locally.

Consider the following `spin.toml` file for the application:

```toml
spin_version = "1"
authors = ["Radu Matei <radu.matei@fermyon.com>"]
description = ""
name = "github-stars-webhook"
trigger = { type = "http", base = "/" }
version = "0.1.0"

[[component]]
id = "github-star-webhook"
source = "target/spin-http-js.wasm"
files = ["my-file.json"]
allowed_http_hosts = ["https://hooks.slack.com"]
[component.trigger]
route = "/..."
```

Distributing it to the GitHub Container Registry:

```bash
$ spin oci push ghcr.io/radu-matei/spin-example:v1
INFO spin_publish::oci::client: Pushed "https://ghcr.io/v2/radu-matei/spin-example/manifests/sha256:8f86a27fbc457416701c4d18680083f598076d0a52dca2a5936e92754a845ed1"

$ spin oci pull ghcr.io/radu-matei/spin-example:v1
INFO spin_publish::oci::client: Pulled ghcr.io/radu-matei/spin-example:v1@sha256:8f86a27fbc457416701c4d18680083f598076d0a52dca2a5936e92754a845ed1
```

This operation created the following local cache directory structure:

```bash
$ tree /Users/radu/Library/Application\ Support/fermyon/registry
└── oci
    ├── data
    │   └── sha256:a4699e4f9ef3f4922f38f0d017aa26438908f38caf020a739e0ee27fe796eb02
    ├── manifests
    │   └── ghcr.io
    │       └── radu-matei
    │           └── spin-example
    │               └── v1
    │                   ├── config.json
    │                   └── manifest.json
    └── wasm
        └── sha256:55c29ad4b0ad0c6bd8ec1ffc8f04e63342e5901280037ef706b1b114475d3cbb
```

Looking at `manifest.json`, we see the top-level media type for the artifact
configuration to be `application/vnd.fermyon.spin.application.v1+config`

```json
{
  "schemaVersion": 2,
  "config": {
    "mediaType": "application/vnd.fermyon.spin.application.v1+config",
    "digest": "sha256:b36160facea3076ad136c09bd4975a429805945ad313b4674363841d5a7f66a0",
    "size": 643
  },
  "layers": [
    {
      "mediaType": "application/vnd.wasm.content.layer.v1+wasm",
      "digest": "sha256:55c29ad4b0ad0c6bd8ec1ffc8f04e63342e5901280037ef706b1b114475d3cbb",
      "size": 2147122
    },
    {
      "mediaType": "application/vnd.wasm.content.layer.v1+data",
      "digest": "sha256:a4699e4f9ef3f4922f38f0d017aa26438908f38caf020a739e0ee27fe796eb02",
      "size": 178
    }
  ]
}
```

Note: the media types used are not final and *can* change based on the community
standardization for Wasm modules in OCI registries. Expanding on this, there have
been several efforts to distribute Wasm *modules* using OCI registries,
each with its own media type:

* [`wasm-to-oci`](https://github.com/engineerd/wasm-to-oci) and
[`oci-distribution`](https://github.com/krustlet/oci-distribution)
use `application/vnd.wasm.content.layer.v1+wasm`
* [`solo-io/wasm/spec`](https://github.com/solo-io/wasm/blob/master/spec/spec.md)
uses `application/vnd.module.wasm.content.layer.v1+wasm`
* Docker's preview appears to distribute them as `application/vnd.docker.container.image.v1+json`

In short, there is no standard at this point. The argument for using
`application/vnd.module.wasm.content.layer.v1+wasm` content media type for *modules*
is the potential introduction of  *components* in the future.

The manifest contains layers for the Wasm module for the component and for the
one static asset referenced by the component. For each additional component, the
Wasm module and static assets would be individual layers in the manifest above.

Let's explore the OCI configuration object referenced in the manifest —
`config.json` - it is a locked application manifest that Spin can use to run the
application from the local cache:

```json
{
  "spin_lock_version": 0,
  "metadata": {
    "description": "",
    "name": "github-stars-webhook",
    "trigger": {
      "base": "/",
      "type": "http"
    },
    "version": "0.1.0"
  },
  "triggers": [
    {
      "id": "trigger--github-star-webhook",
      "trigger_type": "http",
      "trigger_config": {
        "component": "github-star-webhook",
        "executor": null,
        "route": "/..."
      }
    }
  ],
  "components": [
    {
      "id": "github-star-webhook",
      "metadata": {
        "allowed_http_hosts": ["https://hooks.slack.com"]
      },
      "source": {
        "content_type": "application/wasm",
        "digest": "sha256:55c29ad4b0ad0c6bd8ec1ffc8f04e63342e5901280037ef706b1b114475d3cbb"
      },
      "files": [
        {
          "digest": "sha256:a4699e4f9ef3f4922f38f0d017aa26438908f38caf020a739e0ee27fe796eb02",
          "path": "my-file.json"
        }
      ]
    }
  ]
}
```

This is all the information required for Spin to be able to push, pull, then run
an application from an OCI registry.

### `spin oci push`

`spin oci push` is intended to give users the ability to distribute their
application using widely available container registry services, giving them
as much flexibility as possible in order to integrate `spin oci push` and `spin up`
into their *existing* workflows. To this end, it is intended to be as unopinionated
as possible when it comes to versioning and tag mutability.

As a result, `spin oci push` should allow the option to accept a user-defined
name and tag for the artifact pushed to the registry:

```bash
$ spin oci push --file <path to spin.toml> myregistry.com/myusername/myapp:v1
# OR
$ spin oci push --file <path to spin.toml> myregistry.com/myusername/myapp:latest
# OR
$ spin oci push --file <path to spin.toml> myregistry.com/myusername/myapp:v0.1.0+r2d2
```

However, it should also preserve deriving the reference and tag from the `spin.toml`
application name and version. For example, for the following `spin.toml`:

```toml
name = "myregistry.com/myusername/myapp"
version = "1.2.3"
```

Running `spin oci push` should result in the application being pushed without
having to specify the same information on the command line again:

```bash
$ spin oci push --file <path to spin.toml>
... Pushed the application to myregistry.com/myusername/myapp:v1.2.3
# OR
$ spin oci push --file <path to spin.toml> --buildinfo
... Pushed the application to myregistry.com/myusername/myapp:v1.2.3+r2d2
```

Note: deriving the reference and tag from `spin.toml` when no explicit value is passed
on the command line means the application name must contain the fully qualified
reference, and the tag can only be a semantic version.

### Impact to `spin deploy`

While `spin oci push` should offer the most flexibility when pushing and tagging
an application, `spin deploy` is part of an opinionated workflow which could
continue to enforce its tag mutability and versioning strategy.

In line with the gradual approach to this change, `spin deploy` will also change
to publish the application to OCI registries, and by the time this change
will be default, Fermyon Platform and Fermyon Cloud will support this change.

The goal is to have at least one minor release of Spin with `spin oci` before
updating the *default* `spin deploy` behavior to expect an OCI registry.
Once Fermyon Platform or Fermyon Cloud support accepting OCI references,
the intention is to implement `spin deploy --oci` functionality before
updating the default behavior.

### Implementation status

The `spin oci push`, `spin oci pull`, and `spin oci run` commands are currently
implemented in [the prototype](https://github.com/fermyon/spin/pull/1014). 
The implementation uses the[`oci-distribution`](https://github.com/krustlet/oci-distribution) 
crate from Krustlet to interact with a container registry (and currently uses a fork that
should be patched upstream).

The internals of the loaders need additional work before being merged, and the
migration tool from Bindle to OCI has not been started. The `spin oci login`
command has not been implemented.

## FAQ

### Q: How does this effort relate to the [Bytecode Alliance registry project](https://warg.io/)

A: The maintainers of Spin are some of the creators of the registry effort in
the Bytecode Alliance - one of the project's goals is to be able to use existing
storage mechanisms as the storage backends, including OCI registries. Once the
Bytecode Alliance registry project is mature, the Spin project plans to support
it as a distribution mechanism.

