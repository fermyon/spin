title = "SIP xxx - Plugin System for Spin"
template = "main"
date = "2022-08-09T13:22:30Z"
---

Summary: Plugin system for spin.

Owner: karthik.ganeshram@fermyon.com

Created: August 9, 2022 

## Background 

As the functionality fo spin gets extended, there will be a point where every feature will not be required/used by every user. An example of this would be different triggers for spin along with possible subcommands to add more functionality to spin. Therefore it would make sense to be able to add features as plugins allowing for addition of only the required features witout modifying the spin binary.

## Proposal

Create a new subcommand for spin called `spin plugin` which will have two further subcommands namely

- `install`
- `uninstall`

### Types of plugins

![Classification of types of plugins](https://i.imgur.com/Fo7EGPQ.png)

- Standalone Binary Plugins (external subcommands) - These work to add features to spin and do not need any use of the spin binary.
- Internal Spin Plugins - These are plugins like triggers that are used by the spin application. 
- Spin dependant User Plugins - These user invoked plugins require some functionality provided by spin.

### Proposed workflow

A plugin can be installed using the following command.

```bash
$ spin plugin install <name_of_plugin>
The following $plugin from <source_of_plugin> will be installed. 
Make sure you trust this source! Continue? <y/N> 
```

This installs the plugin to a to a spin managed directory based on the operating system.

In the case of standaone binary plugins and spin dependant user plugins, the plugin can be invoked as 

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

The execution of the plugins works differently based on the type of plugin.

- **Standalone Binary Plugin** 
  - Once the plugin is invoked using `spin $plugin <args_to_plugin>`, the external subcommand is invoked as `spin-$plugin <args>`.
- **Spin Dependant User Plugins** 
  - Spin first provides all the functionality available in it required by the plugin and then proceeds to call the plugin along with the required parameters.

- **Spin Dependant Internal Plugins**
  - These plugins cannot be directly invoked by the user as it is used by spin internally. Therefore, only spin invokes these.  

## Proposed implementation of `spin plugin`

### The `install` subcommand

When the `spin plugin install $plugin` command is invoked, spin uses a centralized repository to obtain information about the requested plugin.

Once the information such as the source and version of the plugin is obtained, spin proceeds to download and place the bianry in a spin managed directory.  

#### Update of plugins

The system will use a rolling updates system where the latest plugin will be installed.

### Centralized plugin Repository

- A new github reposity `spin-plugins` will act as the index for all the plugins.
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

The advantages of this method is that it will allow users to be able to search for plugins and imporve visibility of plugins. This will also allow for potentially implementation a `spin plugin search` subcommand in the future.

#### General Naming Conventions of the plugins

- The naming conventions for the binary in the releases must follow certain requirements:
    - The release binary name should be `spin-$plugin`. For windows, it must contain the extension `.exe`.
    - The release binary should not contain any version number in its name.

#### Naming Conventions  of Spin Dependant plugins

Spin dependant plugins can be distinguished from standalone binary plugins based on the name of the plugin. The Spin Dependant Plugins must follow the additional naming constraints.

- The name of the plugin must be `spin-$plugin` where `$plugin` must be `<type_of_plugin>-<name_of_plugin>`

Spin dependant plugins will be categorized based on the setup/functionality sharing required from the spin binary. Based on this, they will be grouped into `<type_of_plguin>`. Some of the possible types are 

- Host components
- Loaders
- Config providers

## Implementation of the execution `spin $plugin`

### Standalone Binary plugins

The spin binary will just directly call the external binary while passing all the arguments passed to it along with a set of enviroment variables providing information about spin.

### Spin dependant binary

For each `<type_of_plguin>`, a new spin execution path will be created sharing the required features/functionality before calling the spin plugin. 

So once the spin plugin is invoked using `spin $plugin`, the spin binary does all the functions defined for that specific `<type_of_plugin` before calling `spin-$plugin` with all the required environment variables set.

## Future Considerations

- Updates with breaking changes.
- Adding support for wasm plugins.
- Adding `search`/`list` subcommands for `spin plugin`.