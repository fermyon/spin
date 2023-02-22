# Modsurfer Module Validation

Using the [Modsurfer](https://github.com/dylibso/modsurfer) tool to validate and scan your Spin
modules is simple. Use the CLI or the [GitHub Action](https://github.com/modsurfer-validate-action) 
to ensure compatibility with the Fermyon Cloud or self-hosted Platform, and check for security or
performance concerns before you deploy your code.

The easiest way to start is by using the GitHub Action. Add the following to your project repository:

#### `./github/workflows/modsurfer.yml`

```yaml
name: Modsurfer Validate - Fermyon
on: [push, pull_request]
jobs:
  check-validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: modsurfer validate
        uses: dylibso/modsurfer-validate-action@main
        with:
            path: path/to/your/module.wasm
            check: mod.yaml
```

And include a "checkfile" in a file called `mod.yaml` (or whichever file you've referenced in the `check` field above):

```yaml
validate:
  url: https://raw.githubusercontent.com/fermyon/spin/main/tools/modsurfer/http/mod.yaml
```

The checkfile above uses a remote reference to ensure your Fermyon Spin project is compatible with 
the latest requirements of the Spin SDKs. This is based off the "http" templates. If you are using
a different template, such as "redis", then find the related checkfile that matches the template 
you're using. 