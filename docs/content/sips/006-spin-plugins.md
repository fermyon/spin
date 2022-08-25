title = "SIP 006 - Spin Plugins"
template = "main"
date = "2022-08-23T14:53:30Z"
---

Summary: A Spin CLI command that will enable plugging in additional functionality and subcommands to Spin.

Owners: karthik.ganeshram@fermyon.com and kate.goldenring@fermyon.com 

Created: August 23, 2022

## Background

The realm of possibilities with Spin continues to grow. However, not every new feature is desired by every user. Instead of needing to modify the Spin codebase, contributors should be able to plug in new functionality or subcommands to Spin via the Spin CLI. This makes Spin easily extensible while keeping it lightweight.

## Proposal

Create a `spin plugin` command, which can be used to install a subcommand that can later be invoked via the Spin CLI. 

For the initial proposal, all Spin plugins are expected to be packaged as an executable that will be executed by Spin when the plugin subcommand is invoked.

A [`spin-plugins` repository](#centralized-plugin-manifest-repository) will act as an inventory of available plugins, made by both Spin maintainers and the community. In the repository, a plugin will be defined by a [JSON Spin plugin manifest](#spin-plugin-manifest). Spin will pull down this manifest during installation, which will instruct it on where to find the plugin binary, version, platform compatibility, and more.

### Usage

The `spin plugin` command will have three sub-commands.

```bash
Commands for working with Spin plugins.

USAGE:
    spin plugin <SUBCOMMAND>

SUBCOMMANDS:
    install      Install plugin as described by a remote or local plugin manifest.
    uninstall    Uninstall a plugin.
    upgrade      Upgrade one or all plugins to the latest or specified version.
```

**`spin plugin install`**

The `spin plugin install` subcommand installs a plugin named `$name`. By default, it will look for a plugin manifest named `$name.json` in the `spin-plugin` repository; however, it can be directed to use a local manifest or one at a different remote location using the `--file` or `--url` flag, respectively. 

> Note: the plugin `$name` must not match an existing internal Spin command name. For example, `spin plugin install up` would elicit an error.

```bash
Install a Spin plugin using a plugin manifest file. 
By default, looks for the plugin manifest named <name>.json
in the Spin plugins repository https://github.com/fermyon/spin-plugins.

USAGE:
    spin plugin install <name>

OPTIONS:
    -f, --file                       Path to local plugin manifest.
    -u, --url                        Address of remote plugin manifest.
    -v, --version                    Version of plugin to be installed. Defaults to latest.
    -y, --yes                        Assume yes to all queries.
```

If the manifest is found, Spin will check that the plugin is compatible with the current OS, platform, and version of Spin. If so, before installing the plugin, Spin will prompt the user as to whether to trust the source. For example, the following prompt would be displayed for a plugin named `test` with an Apache 2 license and hosted at `https://github.com/fermyon/spin-plugin-test/releases/download/v0.1.0/spin-plugin-test-v0.1.0-macos-aarch64.tar.gz`:

```bash
Installing plugin `test` with license Apache 2.0 from https://github.com/fermyon/spin-plugin-test/releases/download/v0.1.0/spin-plugin-test-v0.1.0-macos-aarch64.tar.gz
For more information, reference the plugin metadata at `https://github.com/fermyon/spin-plugins/plugin-manifests/test.json`.
Are you sure you want to proceed? (y/N)
```

The plugin will only be installed if a user enters `y` or `yes` (ignoring capitalization). Otherwise, the command exits. 

Spin will reference the plugin manifest in order to fetch the plugin binary and install it into the user’s local data directory under a Spin-managed `plugins` subdirectory. The plugin manifest will be stored within a `manifests` subdirectory. 

After installing a plugin, it can be executed directly from the Spin CLI. For example, a plugin named `$name` would be executed by running `spin $name <args>`. Any additional arguments supplied will be passed when executing the associated binary.


**`spin plugin uninstall`**

The `spin plugin uninstall` command uninstalls a plugin named `$name`. 

```bash
Uninstall a Spin plugin.

USAGE:
    spin plugin uninstall <name>
```

**`spin plugin upgrade`**

The `spin plugin upgrade` command upgrades one or all plugins. If upgrading a single plugin, the desired version can be specified. By default, plugins are upgraded to the latest version in the plugins repository. As with `spin plugin install`, the local path or remote addresses to a plugin manifest can be specified.

```bash
Upgrade one or all installed Spin plugins.

USAGE:
    spin plugin upgrade [OPTIONS]

OPTIONS:
    -a, --all        Upgrade all installed plugins to latest versions (cannot be used with any other option).
    -p, --plugin     Name of plugin to upgrade.
    -v, --version    Desired version to upgrade the plugin to. Defaults to latest. 
    -f, --file       Path to local manifest (mutex with `-u`).
    -u, --url        Address of remote manifest (mutex with `-f`).
    -d, --downgrade  Enables downgrading a plugin to an older specified version.
```

The upgrade will fail if the latest or user-specified version of the plugin is not [compatible with the current version of Spin](#plugin-compatibility).

### Spin Plugin Manifest

A Spin plugin is defined by a Spin Plugin Manifest which is a JSON file that conforms with the following [JSON Schema](https://json-schema.org/):

```json
{
    "$schema": "https://json-schema.org/draft/2019-09/schema",
    "$id": "https://github.com/fermyon/spin-plugins/json-schema/spin-plugin-manifest-schema-0.1.json",
    "type": "object",
    "title": "spin-plugin-manifest-schema-0.1",
    "required": [
        "name",
        "description",
        "version",
        "spinCompatibility",
        "license",
        "packages"
    ],
    "properties": {
        "name": {
            "type": "string"
        },
        "description": {
            "type": "string"
        },
        "homepage": {
            "type": "string"
        },
        "version": {
            "type": "string"
        },
        "spinCompatibility": {
            "type": "string",
            "pattern": "^([><~^*]?[=]?v?(0|[1-9]\\d*)(\\.(0|[1-9]\\d*))?(\\.(0|[1-9]\\d*))?(-(0|[1-9]\\d*|\\d*[a-zA-Z-][0-9a-zA-Z-]*)(\\.(0|[1-9]\\d*|\\d*[a-zA-Z-][0-9a-zA-Z-]*))*)?(\\+[0-9a-zA-Z-]+(\\.[0-9a-zA-Z-]+)*)?)(?:, *([><~^*]?[=]?v?(0|[1-9]\\d*)(\\.(0|[1-9]\\d*))?(\\.(0|[1-9]\\d*))?(-(0|[1-9]\\d*|\\d*[a-zA-Z-][0-9a-zA-Z-]*)(\\.(0|[1-9]\\d*|\\d*[a-zA-Z-][0-9a-zA-Z-]*))*)?(\\+[0-9a-zA-Z-]+(\\.[0-9a-zA-Z-]+)*)?))*$"
        },
        "license": {
            "type": "string"
        },
        "packages": {
            "type": "array",
            "minItems": 1,
            "items": {
                "type": "object",
                "required": [
                    "os",
                    "arch",
                    "url",
                    "sha256"
                ],
                "properties": {
                    "os": {
                        "type": "string",
                        "enum": [
                            "linux",
                            "osx",
                            "windows"
                        ]
                    },
                    "arch": {
                        "type": "string",
                        "enum": [
                            "amd64",
                            "aarch64"
                        ]
                    },
                    "url": {
                        "type": "string"
                    },
                    "sha256": {
                        "type": "string"
                    }
                },
                "additionalProperties": false
            }
        }
    },
    "additionalProperties": false
}
```

A plugin manifest defines a plugin’s name, version, license, homepage (i.e. GitHub repo), compatible Spin version, and gives a short description of the plugin. It also points to the plugin source for various operating systems and platforms.

The `name` and `spinCompatibility` fields have specific format conventions.

#### Spin Plugin Naming Conventions

The following naming conventions are to be followed for plugins where `$name` is the name of the plugin.

- The `name` field in the plugin manifest must be `$name`.
- Even if the majority of plugins live within the Spin plugins repository, there is a need to distinguish between plugins that are maintained by Spin vs community plugins. They will be distinguished via the plugin name inside the manifest. The name of community plugins must not have "spin" as a prefix, while plugins maintained by Spin should contain a prefix of `spin-`.
- Manifests for older versions of the plugin can be retained in the Spin Plugins repository named `$name@$version.json` where `$version` is the value of the `version` field of the manifest. These specific versions can be installed using the `--version` flag.
- The binary of the plugin must be named `$name`
- The latest plugin manifest file must be named `$name.json`
- The license for the plugin must be named `$name.license`

#### Plugin Compatibility

Spin plugins must specify compatible versions of Spin in the `spinCompatibility` field of the manifest. The field is expected to be a list of rules, with each rule being a [comparison operators](https://docs.rs/semver/1.0.13/semver/enum.Op.html) (`=, >, >=, <, <=, ~, ^, *`) along with the compatible version of Spin. The JSON schema validates that the `spinCompatibility` field is a string that matches the following regular expression: `^([><~^*]?[=]?v?(0|[1-9]\d*)(\.(0|[1-9]\d*))?(\.(0|[1-9]\d*))?(-(0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(\.(0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*)?(\+[0-9a-zA-Z-]+(\.[0-9a-zA-Z-]+)*)?)(?:, *([><~^*]?[=]?v?(0|[1-9]\d*)(\.(0|[1-9]\d*))?(\.(0|[1-9]\d*))?(-(0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(\.(0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*)?(\+[0-9a-zA-Z-]+(\.[0-9a-zA-Z-]+)*)?))*$`. 
For example, specifying `=0.4` means that the plugin is compatible with versions equivalent to `>=0.4.0, <0.5.0`. Multiple rules may be specified (i.e. `>=0.2, <0.5`).

Spin will use the [`semver`](https://docs.rs/semver/1.0.13/semver/struct.VersionReq.html) crate that inspired this syntax to verify that the plugin works on the current version of Spin. If it does not, it will fail to install the plugin and log a message explaining the version mismatch.

#### Centralized Plugin Manifest Repository

- A new GitHub repository https://github.com/fermyon/spin-plugins will act as the index for all the Spin plugin manifests. Having a centralized location for plugin manifests enables future support of a `spin plugin search` subcommand that allows users to search for plugins via the Spin CLI.
- Creators of new plugins can submit PRs to add a plugin manifest to the repository.
- Plugin creators are required to test Spin compatibility with their plugin and update the `spinCompatability` field of the manifest over time accordingly.
- Plugin manifests can be hosted elsewhere and installed via the `--file` or `--url` fields of `spin plugin install`.

## Future design considerations

### Larger scope for Spin Plugins

The concept of Spin plugins is to allow both new subcommands and functionality to be added to Spin. This SIP focuses on the former, enabling users to both install and execute subcommands from the Spin CLI; however, there are cases where it may be useful to install a new Spin feature that is executed by Spin rather than the user. An example of this is Spin triggers. A user may wish to [extend Spin to support a timer trigger](https://spin.fermyon.dev/extending-and-embedding/) that executes components at a configured time interval. Instead of having to understand, modify, and grow the spin codebase, a user could package the trigger as a plugin. After installing the trigger via `spin plugin install`. Spin could invoke it when a Spin manifest references the trigger. 

### WebAssembly Plugin Support
While for now plugins are assumed to be executables, in the future, support for plugging in WebAssembly modules may be desirable.

### Clean versioning and Spin plugin compatibility

The proposed method of using version strings to declare compatibility between a plugin and Spin has several drawbacks. Firstly, this requires plugin creators to stay involved with their contribution, regularly testing and updating the compatibility of their plugin with Spin. One way to make this more hands-off would be to encourage plugin creators to also contribute an integration test. For each new spin release, a workflow in the plugins repository can automatically run these integration tests and bump compatibility versioning on success. This is a strategy taken by [MicroK8s](https://microk8s.io/docs/addons) for its core and community add-ons.

Another issue with using versioning to check for compatibility with Spin is that canary releases of Spin have the same version as the latest release. This means that if a user is using main or canary Spin, when Spin checks its version before installing a plugin, it may incorrectly assume compatibility even though its feature set is beyond that of the latest stable release. Spin templates currently have a workaround for detecting and handling this inconsistency. A more ideal way of assessing compatibility would be via capability checking wherein a plugin could declare what set of features it is compatible with and Spin would assert if those exist. For example, a plugin could be compatible with only a specific version of Spin manifests or only have support for WAGI. While a system like this would be full-proof, it would require deep design. As plugins are developed, a better understanding will come of what capabilities plugins need from Spin. From this, compatibility via compatibilities system could be designed.