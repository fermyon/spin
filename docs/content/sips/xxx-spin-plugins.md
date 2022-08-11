title = "SIP xxx - Plugin System for Spin"
template = "main"
date = "2022-08-09T13:22:30Z"
---

Summary: Plugin system for spin.

Owner: karthik.ganeshram@fermyon.com

Created: August 9, 2022 

## Background 

As the functionality fo spin gets extended, there will be a point where every feature will not be required/used by every user. An example of this would be different triggers for spin along with possible subcommands to add more functionality to spin. Therefore it would make sense to be able to add features as plugins allowing for addition of only the required features without modifying the spin binary.

## Proposal

Create a new subcommand for spin called `spin plugin` which will have two further subcommands namely

- `install`
- `uninstall`

### Types of plugins
```text
Spin Plugins
├── Spin Dependant Plugins
│   ├── Spin-Dependant Internal Plugins
│   └── Spin-Dependent User Plugins
└── Standalone Binary Plugins
```
**Standalone Binary Plugin** - These are just basic binaries that can add functionality to spin (eg) spin routes just displays the HTTP routes for a given application from spin.toml.

**Spin-based User Plugins** - A made-up example of this would be spin deploy where the user invokes the command but as a plugin, it would require some functionality from spin. Therefore deploy is a type of command and it could have multiple plugins each of which deploys to different services.

**Spin-based Internal Plugins** - An example of this would be triggered, where the user does not directly call for the triggers but uses them by specifying them in the application manifest. Loaders would be another example of this.

### Proposed workflow

A plugin can be installed using the following command.

```bash
$ spin plugin install <name_of_plugin>
The following $plugin from <source_of_plugin> will be installed. 
Make sure you trust this source! Continue? <y/N> 
```

This installs the plugin to a to a spin managed directory based on the operating system.

In the case of standalone binary plugins and spin dependant user plugins, the plugin can be invoked as 

```bash
$ spin $plugin <args_to_plugin>
```

In the case of spin Dependant Internal Plugins, the user does not directly invoke the command, these plugins are directly invoked by spin where required (i.e) like triggers based on the application manifest. [(Spin Trigger Executor)](https://spin.fermyon.dev/sips/003-trigger-executors.md)

To uninstall plugins, the following command can be used.

```
spin uninstall $plugin
```

This will uninstall the executable from the spin managed directory.


### Execution of plugins

The execution of the plugins works differently based on the type of plugin. Spin would be able to identify the type of plugins based on the name of the plugin.

- **Standalone Binary Plugin** 

Once the plugin is invoked using `spin $plugin <args_to_plugin>`, the external subcommand is invoked as `spin-$plugin <args>` along with information about spin passed through environmental variables.

- **Spin-Based Plugins**

Both the types of Spin-Based plugins have the same flow of execution but differ in how spin launches them. The Spin-Based plugins are divided into different `types` based on the different requirements of different plugins. Some examples of different `types` of plugins would be `trigger`, `loader`, `deploy` etc. These are well known strings.

These `types` are then grouped up into 2 categories called User Plugins and Internal Plugins depending on how they are invoked. Internal Plugins are called implicitly by application configuration while the User plugins are explicitly invoked by the user.

Based on the type of plugin, Spin knows the execution path that needs to be followed before invoking the plugin (i.e) the environment variables that it sets and other things like placing files in appropriate directories. In the case of Internal plugins, the would also define how the data must be returned if any is required. 

  - **Spin Dependant User Plugins** 
These plugins are explicitly called by the user. An example of this would the `deploy` type of plugins (i.e) A potential plugin that deploys to application to bindle would be called using

```
spin deploy-bindle
```

where `deploy` would be the type of the plugin and spin would know the prerequisites that it must provide before invoking the plugin.

  - **Spin Dependant Internal Plugins**
These plugins are not explicitly called by the user but are implied implicitly based on the contents of `spin.toml`. (e.g) A trigger plugin based on time would be defined in the `spin.toml` as 

```rust
trigger = {type = "spin-trigger-time"}
```

In this case, Spin figures out that it needs to use an Internal Plugin to satisfy the requirements of the application. `trigger` here is an example of Internal Plugin `type`. 

## Proposed implementation of `spin plugin`

### The `install` subcommand

When the `spin plugin install $plugin` command is invoked, spin uses a centralized repository to obtain information about the requested plugin.

Once the information such as the source and version of the plugin is obtained, spin proceeds to download and place the binary in a spin managed directory along with the license.  

#### Installing from other sources

Optional arguments can be created to allow installation from a local directory or form a custom git repository.

```
# installing from local source
spin plugin install -f <path_to_tar> <plugin_name>
# installing from custom Git repo
spin plugin install --custom <url_to_plugin_metadata>
```

#### Update of plugins

The plugins can be updated using the `--update` flag on the `install` subcommand. The `--update` argument will update a package to the newest version if it already exists or else will install the newest version.

### Centralized plugin Repository

- A new github repository `spin-plugins` will act as the index for all the plugins.
- It will contain information about the plugins such as location of binary release for the various operating system, version and checksum along with potentially other details such as author details and a description.
- Creators of new plugins can submit PRs with the required information to add it to the plugins index repository.

The structure of data stored for every plugin is a json file with the following structure

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
            "os": "macOs",
            "arch": "aarch64",
            "url": "<url_to_plugin_tar>",
            "sha256": "18f023db8b39b567a6889fa6597550ab2b47f7947ca47792ee6f89dfc5a814e3"
        }
    ]
}
```

The `packages` field is a list of all the package variants available for the plugin based on the different OS and architecture of the system. Not all plugins need to be able to run for all variants.

The advantages of this method is that it will allow users to be able to search for plugins and improve visibility of plugins. This will also allow for potentially implementation a `spin plugin search` subcommand in the future.

The versioning of plugins needs to be discussed. Potential options are
- Maintain the only the latest release of each major version, where the latest version of the plugin will be name `$plugin` while the older version will use the naming scheme `$plugin@<version>`.
- Maintain all the releases and by default install the latest version according to `SemVer` but also provide a `--version` flag to specify particular versions.  

#### General Naming Conventions of the plugins

- The naming conventions for the binary in the releases must follow certain requirements:
    - The release binary name should be `spin-$plugin`. For windows, it must contain the extension `.exe`.
    - The release binary should not contain any version number in its name.

#### Naming Conventions  of Spin Dependant plugins

Spin dependant plugins can be distinguished from standalone binary plugins based on the name of the plugin. The Spin Dependant Plugins must follow the additional naming constraints.

- The name of the plugin must be `spin-$plugin` where `$plugin` must be `<type_of_plugin>-<name_of_plugin>`

Spin dependant plugins will be categorized based on the setup/functionality sharing required from the spin binary. Based on this, they will be grouped into `<type_of_plugin>`. Some of the possible types are 

- Host components
- Loaders
- Config providers

## Implementation of the execution `spin $plugin`

### Standalone Binary plugins

The spin binary will just directly call the external binary while passing all the arguments passed to it along with a set of environment variables providing information about spin.

### Spin dependant binary

For each `<type_of_plugin>`, a new spin execution path will be created sharing the required features/functionality before calling the spin plugin. 

So once the spin plugin is invoked using `spin $plugin`, the spin binary does all the functions defined for that specific `<type_of_plugin` before calling `spin-$plugin` with all the required environment variables set.

## Future Considerations

- Adding support for wasm plugins.
- Adding `search`/`list` subcommands for `spin plugin`.