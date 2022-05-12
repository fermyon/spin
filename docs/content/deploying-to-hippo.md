title = "Deploying Spin applications to Hippo"
template = "main"
date = "2022-05-12T17:41:53.438445Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/deploying-to-hippo.md"

---

The Spin CLI offers a command to deploy a spin application to the Hippo platform.

## Deploying Spin applications to Hippo

[Hippo](https://github.com/deislabs/hippo) is a Platform as a Service (PaaS) for running WebAssembly applications. Recently,
Hippo has added support for scheduling WebAssembly modules on Nomad using Spin. The
`spin deploy` command is an experimental command for deploying Spin applications to
Hippo locally or in the cloud. This document provides guidance for how to get set up
locally. Documentation on how to get set up to deploy to the cloud using `spin deploy` is
coming soon.

### Pre-requisites

- [Spin <= v0.2.0](https://github.com/fermyon/spin/releases)
- [Hippo CLI v0.10.0](https://github.com/deislabs/hippo-cli/releases/tag/v0.10.0)
- [Bindle v0.8.0](https://github.com/deislabs/bindle/releases/tag/v0.8.0)
- [Nomad >= v1.2.6](https://www.nomadproject.io/)
- [Consul >= v1.11.3](https://www.consul.io/)
- [Traefik >= 2.6.1](https://github.com/traefik/traefik/releases)
- [Dotnet 6.0 CLI](https://dotnet.microsoft.com/en-us/download)

### Getting Set Up

Start Nomad, Consul, and Traefik using the `run_servers.sh` script from the [nomad-local-demo repo](https://github.com/fermyon/nomad-local-demo).

```
$ git clone git@github.com:fermyon/nomad-local-demo.git
$ cd nomad-local-demo
$ # you can either checkout this commit or if using HEAD, comment out the Hippo job in the
$ #     run_servers.sh script
$ git checkout 64cf9334528f1975d7cbff207997d83cee4f19c2
$ ./run_servers.sh
```

Clone the Hippo repo locally and checkout the v0.6.2 tag. Set the `BINDLE_URL` environment variable from the `run_servers.sh` script. For now, we'll need to manuall update a few lines in the Nomad job in the project and then, run the `dotnet build` command to build the project. Change into the `src/Web` directory and use `dotnet run` to run the project.

```
$ git clone git@github.com:deislabs/hippo.git
$ cd hippo
$ git checkout tags/v0.6.2
```

In `src/Infrastructure/Jobs/NomadJob.cs`, comment out the artifact stanza on [lines 191-195](https://github.com/deislabs/hippo/blob/v0.6.2/src/Infrastructure/Jobs/NomadJob.cs#L191-L195).
This will ensure that Hippo uses the Spin binary on your machine instead of downloading a specific version.

If you are using a mac and not a linux machine, also change the `driver` value on [line 189](https://github.com/deislabs/hippo/blob/v0.6.2/src/Infrastructure/Jobs/NomadJob.cs#L189) from `driver = ""exec""` to `driver = ""raw_exec""`

_Note: These manual job changes are updated and unnecessary using the latest version of Hippo and Spin will update to use a newer version of Hippo soon_

```
$ export BINDLE_URL=http://bindle.local.fermyon.link:8088/v1
$ dotnet build
$ cd src/Web
$ dotnet run \
  --Scheduler:Driver=nomad \
  --Bindle:Url="${BINDLE_URL}"
```

Check that the Hippo platform is running by making sure you see the hippo dashboard in the browser at https://localhost:5309.

Register an account in Hippo using the Hippo CLI.

```
hippo auth register --username user --password PassW0rd! --url https://localhost:5309 -k
```

### Deploy the example http-rust app

Clone the [Spin repo](https://github.com/fermyon/spin) to ensure you have the examples locally. Make sure that the http-rust example is built before deploying.

```
$ git clone git@github.com:fermyon/spin.git
$ cd spin/examples/http-rust
$ spin build
```

Then, set the relevant environment variables and use `spin deploy` to deploy the app to Hippo. _Note: you can also use flags to configure spin deploy_

```
$ export HIPPO_URL=https://localhost:5309
$ export BINDLE_URL=http://bindle.local.fermyon.link:8088/v1
$ export HIPPO_USERNAME=user
$ export HIPPO_PASSWORD=PassW0rd!

$ spin deploy

```

The spin deploy command packages the application and pushes it to the bindle registry, creates a new app in Hippo and a new channel that the runs the application.

### Test the app

In the future, spin deploy will give the user a domain to hit for the running app. In the meantime, find the IP address and port for the running app int he Nomad dashboard.

Check the Nomad UI for the running job.

```
# this command opens up the nomad dashboard in your browser
$ nomad ui
```

Inspect the job for the IP address and port for the running
application and hit the `/hello` route to see `Hello Fermyon!`.
