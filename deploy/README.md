# Deployments

The [Spin Docs](https://spin.fermyon.dev) website is deployed via the [deploy-website.yaml](../.github/workflows/deploy-website.yml) GitHub workflow.

(Note: currently this website consists of redirects to the Spin Docs hosted on Fermyon's [Developer site](https://developer.fermyon.com/spin))

## Auto Deploys

The production version of the website is deployed whenever commits are pushed to the `main` branch.

## Manual Deploys

Deployments may also be [triggered manually](https://github.com/fermyon/spin/actions/workflows/deploy-website.yml), providing a choice of `ref` and `sha`.

## Nomad jobs

We currently deploy the website via its Nomad job directly. (In the future, we envision running the website as a Fermyon Cloud app.)

The [spin-docs](./spin-docs.nomad) Nomad job contains configuration for the running website, including the OCI reference to run from.