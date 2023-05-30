title = "SIP 014 - "Cloud Plugin"
template = "main"
date = "2023-05-10T23:09:00Z"

---

Summary: This improvement proposal describes the plan to move the cloud-specific
functionality from the core Spin command-line interface to a separate plugin.

Owners: matt.fisher@fermyon.com

Created: May 10, 2023

## Background

The intended outcome of this SIP is to provide a set of recommendations to move
the core logic of `spin login` and `spin deploy` into a separate `spin cloud`
plugin, while continuing to provide a great out-of-the-box experience for those
wanting to deploy their applications to the Fermyon Cloud.

## Proposal

This document proposes to move two commands from the Spin command-line interface
to a separate plugin:

1. `spin login`
2. `spin deploy`

These commands will be moved to a separate `spin cloud` plugin. This document
will also recommend possible approaches to ensure these commands remain easily
accessible to new and existing users of the Spin command-line interface.

This enables the Spin team to release the core Spin runtime and the Spin
command-line interface as stable, while also enabling the functionality of the
Fermyon Cloud to change and iterate over time.

## Rationale

Several commands were introduced to the Spin CLI, making it very simple for
someone to deploy their application to the Fermyon Cloud:

- `spin login`
- `spin deploy`

These commands are orthogonal to the concerns of Spin's core runtime. These
commands involve the packaging and distribution of a Spin application to the
Fermyon Cloud. They are considered "adjacent" to the Spin user experience as
they do not assist the developer with writing their application, nor do they
relate to Spin's core runtime; they provide a simple experience to ship their
application to the Fermyon Cloud.

`spin login` and `spin deploy` communicate with the Fermyon Cloud API. As new
features are introduced to the Fermyon Cloud, the API may change and evolve over
time.

Building a simple, delightful on-ramp experience for the Fermyon Cloud remains
top priority. It is especially important that we continue to provide a very
simple on-ramp experience so users can readily deploy their Spin applications to
a production-grade system.

Spin’s existing plug-in system allows the Spin community to add and extend the
functionality of Spin without introducing changes to the Spin command-line
interface. Plug-ins are described in more detail in [SIP 006 - Spin
Plugins](./006-spin-plugins.md). This allows us to ship `spin login` and `spin
deploy` as separate, discrete functionality while ensuring users can still
access its functionality through familiar tooling.

## Specification

The proposal herein suggests that we re-release the core logic of `spin login`
and `spin deploy` under a separate `spin cloud` plugin. Spin will alias the
existing `spin login` and `spin deploy` commands to their respective `spin
cloud` counterparts to retain backwards compatibility.

When a user executes `spin login` or `spin deploy` and the cloud plugin is not
installed on their system, Spin will inform the user that the `spin cloud`
plugin has not been installed, then install the plugin.

```console
$ spin deploy
The `cloud` plugin is required. Installing now.
Plugin 'cloud' was installed successfully!
Uploading cloud_start version 0.1.0+XXXXXXXX...
Deploying...
Waiting for application to become ready... ready
Available Routes:
  cloud-start: https://cloud-start-xxxxxxxx.fermyon.app/ (wildcard)
```

## Future design considerations

### `spin cloud config`

An early prototype of the `spin cloud` plugin proposed several changes to the
`spin cloud deploy` command, including a new `spin cloud config` command. The
proposed command would configure the "current" Spin app for deployment with an
interactive prompt. This command would be optional, and calling `spin cloud
deploy` on an application that was not yet configured would invoke `spin cloud
configure` in the background. The experience looked something like this:

```console
$ spin cloud deploy
The current Spin app hasn't been set up for deployment yet.
Pick a new or existing deployment config:

> Fermyon Cloud - New Deployment
  Fermyon Cloud - 'spicy-meatballs'

Creating new deployment...
New deployment created: 'mighty-hamster'

[...SNIP normal deploy flow...]
```

While the move to a separate `spin cloud` plugin does not reject this idea
outright, the goal of this SIP is to make the least invasive change to the
existing `spin login` and `spin deploy` experience. Future iterations to the
`spin login` and `spin deploy` experience can be addressed in future updates to
the plugin.

In fact, the movement to a separate `spin cloud` plugin grants us the
flexibility to make changes to the core `spin deploy` experience without forcing
us to wait until Spin 2.0. If anything, this proposal enables us to make these
changes to the `spin cloud` plugin without waiting for a new release of Spin.

### A generic `spin cloud` plugin supporting multiple cloud providers

One recommendation was to design `spin cloud` in a generic fashion. In this
manner, "cloud providers" (including Fermyon Cloud) would integrate with `spin
cloud`. Customers would use `spin cloud login` and `spin cloud deploy` to
deploy Spin applications to their hosting platform of choice.

Rather than a generic plugin that prescribes a command flow for all clouds, we
hope partners come to us to add their own plugin for deploying Spin applications
to their cloud (and we are open to the idea of collaborating on such a
project!). For the initial launch, the goal of this SIP is to make the least
invasive change to the existing `spin login` and `spin deploy` experience.

### Will users be able to run CRUD operations on Cloud specific objects (like a KV store)?

The goal of this SIP is to find a new home for `spin login` and `spin deploy`.
Future iterations to the `spin cloud` plugin (such as a `spin cloud kv` command)
may be provided in future updates to the plugin.

## Open Questions

> How do we ensure the `spin cloud` plugin is kept up-to-date? Do we ask the
> user to run `spin plugin update`? Do we inform them that a new version of the
> plugin is available? Do we update the plugin for the user?

The goal of this SIP is to find a new home for `spin login` and `spin deploy`.
Future iterations to the plugin system can be handled separately as a feature
enhancement. For the time being, asking the user to run `spin plugin update`
aligns with the current plugin model.

> How do we install the plugin? Install on first invocation of `spin login` or
> `spin deploy`? Install the first time the user runs a regular `spin` command?

Per the current plugin system, we will return an error when the user attempts to
run a command when the plugin does not exist with a helpful message. Asking the
user to run `spin plugin install` aligns with the current plugin model.
