title = "SIP xxx - Plugin System for Spin"
template = "main"
date = "2022-08-09T13:22:30Z"
---

Summary: Plugin system for Spin.

Owner: karthik.ganeshram@fermyon.com

Created: August 9, 2022 

## Background 

As the functionality of Spin gets extended, there will be a point where every feature will not be required/used by every user. An example of this would be different triggers for Spin along with possible subcommands to add more functionality to Spin. Therefore it would make sense to be able to add features as plugins without modifying the Spin binary.

## Proposal

Create a new subcommand for Spin called `spin plugin` which will have three further subcommands namely:

- `install`
- `uninstall`
- `update`

### Types of plugins
```text
Spin Plugins
├── Spin Dependant Plugins
│   ├── Spin-Dependant Internal Plugins
│   └── Spin-Dependent User Plugins
└── Standalone Binary Plugins
```
**Standalone Binary Plugin** - These are binaries that can add functionality to Spin without requiring functionality from the Spin binary(eg) Spin routes just displays the HTTP routes for a given application from spin.toml.

**Spin-Based User Plugins** - A made-up example of this would be `spin deploy` where the user invokes the command but as a plugin, it would require some functionality from spin. Therefore deploy is a type of command and it could have multiple plugins each of which deploys to different services.

**Spin-Based Internal Plugins** - An example of this would be triggers, where the user does not directly call the triggers but uses them by specifying them in the application manifest. Loaders would be another example of this.

### Proposed Workflow

A plugin can be installed using the following command.

```bash
$ spin plugin install <name_of_plugin>
The following $plugin from <source_of_plugin> will be installed. 
Make sure you trust this source! Continue? <y/N> 
```

This installs the plugin to a to a Spin managed directory based on the operating system.

In the case of Standalone Binary Plugins and Spin Dependant User Plugins, the plugin can be invoked as 

```bash
$ spin $plugin <args_to_plugin>
```

In the case of Spin Dependant Internal Plugins, the user does not directly invoke the command, these plugins are directly invoked by Spin where required (i.e) like triggers based on the application manifest. [(Spin Trigger Executor)](https://spin.fermyon.dev/sips/003-trigger-executors.md)

To uninstall plugins, the following command can be used.

```
spin uninstall $plugin
```

This will uninstall the executable from the Spin managed directory.


### Execution of plugins

The execution of the plugins works differently based on the type of plugin. Spin would be able to identify the type of plugins based on the name of the plugin.

- **Standalone Binary Plugin** 

Once the plugin is invoked using `spin $plugin <args_to_plugin>`, the external subcommand is invoked as `spin-$plugin <args>` along with information about Spin passed through environmental variables.

- **Spin-Based Plugins**

Both the types of Spin-Based plugins have the same flow of execution but differ in how Spin launches them. The Spin-Based plugins are divided into different `types` based on the different requirements of different plugins. Some examples of different `types` of plugins would be `trigger`, `loader`, `deploy` etc. These are well known strings.

These `types` are then grouped up into 2 categories called User Plugins and Internal Plugins depending on how they are invoked. Internal Plugins are called implicitly by application configuration while the User plugins are explicitly invoked by the user.

Based on the type of plugin, Spin knows the execution path that needs to be followed before invoking the plugin (i.e) the environment variables that it sets and other things like placing files in appropriate directories.

  - **Spin Dependant User Plugins** 
These plugins are explicitly called by the user. An example of this would the `deploy` type of plugins (i.e) A potential plugin that deploys an application to Bindle would be called using

```
spin deploy-bindle
```

where `deploy` would be the type of the plugin and Spin would know the prerequisites that it must be completed before invoking the plugin.

  - **Spin Dependant Internal Plugins**
These plugins are not explicitly called by the user but are implied implicitly based on the contents of `spin.toml`. (e.g) A trigger plugin based on time would be defined in the `spin.toml` as 

```rust
trigger = {type = "spin-trigger-time"}
```

In this case, Spin figures out that it needs to use an Internal Plugin to satisfy the requirements of the application. `trigger` here is an example of Internal Plugin `type`. 

## Proposed implementation of `spin plugin`

### The `install` subcommand

When the `spin plugin install $plugin` command is invoked, Spin uses a centralized repository to obtain information about the requested plugin. 

Once the information such as the source and version of the plugin is obtained, Spin proceeds to download and place the binary in a Spin managed directory along with the license.  

#### Installing from other sources

Optional arguments allow for installation from a local directory or from a custom remote source. 

```
# installing from local manifest
spin plugin install -f <path_to_plugin_manifest>
# installing from remote manifest
spin plugin install -r <url_to_plugin_manifest>
```

When installing from other sources, an additional argument `--force` or `-f` can allow for the user to force install a plugin allowing users to install specific versions of plugins even if they have the latest version installed.

#### Update of plugins

The plugins can be updated using the `update` subcommand.

```
spin plugin update <name_of_plugin>
```

This will update the package to the newest version if it exists or else will install it.
The `update` subcommand will also have an optional argument `--all` which update all the installed plugins.

**Dealing with breaking changes**

This is a section that still needs discussion. One of the solutions is for the plugin manifest to list the versions of Spin that it is compatible with and use that to make an informed update of plugins avoiding updates to incompatible versions.

### Centralized Plugin Repository

- A new github repository `spin-plugins` will act as the index for all the plugin manifests.
- Each plugin manifest will contain information about the plugins such as location of binary release for the various operating system, version and checksum along with potentially other details such as author details and a description.
- Creators of new plugins can submit PRs with the required information to add it to the plugins index repository.

The structure of data stored for every plugin is a json file with a specified JSON schema. The initial JSON schema will be the following:
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
        "license",
        "packages"
    ],
    "properties": {
        "name": {
            "type": "string",
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

The JSON schema will live with the plugin manifests in the `spin-plugins` repository. A GitHub workflow will validate all plugin manifests against the JSON schema before they are allowed to be merged.

An example plugin manifest that matches the schema is the following:

```
{
    "name": "test",
    "description": "Some description.",
    "homepage": "www.example.com",
    "version": "1.0",
    "license": "Mit",
    "packages": [
        {
            "os": "linux",
            "arch": "amd64",
            "url": "<url_to_plugin_tar>",
            "sha256": "18f023db8b39b567a6889fa6597550ab2b47f7947ca47792ee6f89dfc5a814e3"
        },
        {
            "os": "osx",
            "arch": "aarch64",
            "url": "<url_to_plugin_tar>",
            "sha256": "18f023db8b39b567a6889fa6597550ab2b47f7947ca47792ee6f89dfc5a814e3"
        }
    ]
}
```

The `packages` field is a list of all the package variants available for the plugin based on the different OS and architecture of the system. Not all plugins need to be able to run for all variants.

Some additional considerations that need to be taken are:
- Does the plugin need to list the versions of spin it is compatible with?
- Can a plugin depend on other plugins?

The advantages of this method is that it will allow users to be able to search for plugins and improve visibility of plugins. This will also allow for potentially implementation a `spin plugin search` subcommand in the future.

The versioning of plugins needs to be discussed. Potential options are
- Maintain the only the latest release of each major version, where the latest version of the plugin will be name `$plugin` while the older versions will use the naming scheme `$plugin@<version>`. If a requirement for a specific version of the binary arises, the user can choose to install it from a custom source (local or remote).
- Maintain all the releases and by default install the latest version according to `SemVer` but also provide a `--version` flag to specify particular versions.  

#### General naming conventions of plugins

The following naming conventions are to be followed for plugins where `$plugin` is the name of the plugin. 
- The name of the plugin must not have "spin" as a prefix.
- `$plugin` cannot be equal to one of the predefined `types` of plugins.
- The plugins manifest must be named `$plugin.json`
- The name field in the plugin manifest must be `$plugin`.
- The binary of the plugin must be named `spin-$plugin` to distinguish it from other binaries.
- The latest major release binaries must not contain any version numbers.
- For older major releases, the name of the plugin `$plugin` must be suffixed with `@<major_version>`.
- The license for the plugins must be named `$plugin.license`

#### Additional naming conventions for Spin Dependant Plugins

Spin Dependant Plugins can be distinguished from standalone binary plugins based on the name of the plugin. The Spin Dependant Plugins must follow the additional naming constraints.

- The name of the plugin must be `spin-$type-$plugin` where `$type` is one of pre-defined types of plugins.

Spin Dependant Plugins will be categorized based on the setup/functionality sharing required from the Spin binary. The plugins will be grouped together based on `type`. Some of the possible types are 

- Host components
- Loaders
- Config providers
- Deploy
- Trigger

## Packaging of plugins

The plugin must be packaged as `$plugin.tar.gz`. There must be no directories in the archive. The archive must contain the plugin binary according to the naming conventions along with a license and optionally a readme.

An example of a packaging a plugin named `test` would be as follows.

```
tar -czvf $plugin.tar.gz $plugin $plugin.license
```

which will produce `test.tar.gz`.

## Implementation of the execution `spin $plugin`

### Standalone Binary Plugins

The Spin binary will just directly call the external binary while passing all the arguments passed to it along with a set of environment variables providing information about Spin.

### Spin Dependant Plugins

For each `<type>`, a new Spin execution path will be created sharing the required features/functionality before calling the Spin plugin. 

So once the Spin plugin is invoked using `spin $plugin`, the Spin binary does all the functions defined for that specific `<type>` before calling `spin-$plugin` with all the required environment variables set.

## Future considerations

- Adding support for wasm plugins.
- Plugins dependant on other plugins.
- Adding `search`/`list` subcommands for `spin plugin`.