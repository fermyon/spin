# The Spin documentation website

**The Spin documentation website is deprecated.** We still run it, but only to redirect to new documentation.

To build and run the Spin documentation website:

1. Build Spin using the [contributing guide](https://developer.fermyon.com/spin/contributing).

2. Run the website from this directory via Spin:

```
$ spin up
```

3. View documentation website at http://localhost:3000

# Deployments

The [Spin Docs](https://spin.fermyon.dev) website is deployed via the [deploy-website.yaml](../.github/workflows/deploy-website.yml) GitHub workflow.

## Auto Deploys

The website is deployed whenever commits are pushed to the `main` branch.

## Manual Deploys

Deployments may also be [triggered manually](https://github.com/fermyon/spin/actions/workflows/deploy-website.yml).
