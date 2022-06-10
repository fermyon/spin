title = "Deploying Spin applications to Hippo"
template = "main"
date = "2022-05-20T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/deploying-to-hippo.md"
---

The Spin CLI offers a command to deploy a spin application to the Hippo
platform.

## Deploying Spin applications to Hippo

[Hippo](https://github.com/deislabs/hippo) is a Platform as a Service (PaaS)
for running WebAssembly applications. Recently, Hippo has added support for
scheduling WebAssembly modules on Nomad using Spin. The `spin deploy` command
is an experimental command for deploying Spin applications to Hippo locally or
in the cloud.

This document provides guidance for how to get set up locally. Documentation on
how to get set up to deploy to the cloud using `spin deploy` is coming soon.

### Pre-requisites

- [Spin](https://github.com/fermyon/spin/releases)
- [Hippo CLI latest](https://github.com/deislabs/hippo-cli/releases)
- [Bindle v0.8.0](https://github.com/deislabs/bindle/releases/tag/v0.8.0)
- [Nomad >= v1.2.6](https://www.nomadproject.io/)
- [Consul >= v1.11.3](https://www.consul.io/)
- [Vault >= v1.9.3](https://www.vaultproject.io/)
- [Traefik >= 2.6.1](https://github.com/traefik/traefik/releases)

### Getting Set Up

Start Nomad, Consul, and Traefik using the `run_servers.sh` script from the
[nomad-local-demo repo](https://github.com/fermyon/nomad-local-demo).

```
$ git clone git@github.com:fermyon/nomad-local-demo.git
$ cd nomad-local-demo
$ ./run_servers.sh
```

Check that the Hippo platform is running by making sure you see the hippo
dashboard in the browser at <http://hippo.local.fermyon.link>

Register an account in Hippo using the Hippo CLI.

```
$ hippo register --username user --password PassW0rd! --url http://hippo.local.fermyon.link
```

### Deploy the example http-rust app

Clone the [Spin repo](https://github.com/fermyon/spin) to ensure you have the
examples locally. Make sure that the http-rust example is built before
deploying.

```
$ git clone git@github.com:fermyon/spin.git
$ cd spin/examples/http-rust
$ spin build
```

Then, set the relevant environment variables and use `spin deploy` to deploy
the app to Hippo. _Note: you can also use flags to configure spin deploy_

```
$ export HIPPO_URL=http://hippo.local.fermyon.link
$ export BINDLE_URL=http://bindle.local.fermyon.link/v1
$ export HIPPO_USERNAME=user
$ export HIPPO_PASSWORD=PassW0rd!
$ spin deploy
```

The `spin deploy` command packages the application and pushes it to the bindle
registry, creates a new app in Hippo and a new channel that the runs the
application.

### Test the app

In the future, `spin deploy` will give the user a domain to hit for the running
app.

```
$ curl http://spin-deploy.spin-hello-world.hippo.local.fermyon.link/hello
Hello, Fermyon!
```
