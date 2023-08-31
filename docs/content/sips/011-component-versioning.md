title = "SIP 011 - Component versioning"
template = "main"
date = "2023-01-24T01:01:01Z"
---

Summary: Embed version information into Spin components for improved compatibility diagnostics.

Owner(s): joel.dice@fermyon.com

Created: Jan 24, 2023

## Background

Currently, Spin has no way to determine which version of SDK the components of an application were built with, which leads to obscure, hard-to-understand error messages for ABI and/or API incompatibilities.  For example, a component built with a Spin 0.4 SDK won't run on Spin 0.7 due to `wit-bindgen` ABI changes, leading to a variety of confusing runtime errors like out-of-bounds memory traps, assertion errors about invalid values, etc.  In addition, new APIs were added between 0.4 and 0.7, and thus a 0.7 component might not run on 0.4 even if the ABI had not changed.  These errors are usually a bit easier to understand (e.g. missing imports), but still don't point clearly to a resolution.  Ideally, the message presented to the user would be something like, "This component targets Spin 0.7 and cannot be run using this version of Spin (0.4) -- please use Spin 0.7.x to run it."

## Proposal

Although `WIT` does not yet have a way to express API versions, we can emulate it by exporting a set of core WASM functions from the SDK (and thus any component built using that SDK) using the following naming convention:

```
spin-sdk-version-$MAJOR-$MINOR
spin-sdk-language-$LANGUAGE
spin-sdk-commit-$HASH
```

where `$MAJOR` and `$MINOR` are the Spin major and minor version numbers the SDK targets, `$LANGUAGE` is the programming language the SDK supports, and `$HASH` is the Git commit hash the SDK was built from. The intention behind embedding this information in the name of the function is that it can be checked statically without instantiating or running the component, and it is immune to component model ABI changes.

For languages which don't yet have a version-enabled Spin SDK (or developers who wish to use `wit-bindgen` directly, or even implement bindings by hand), the component may itself export any or all of the above functions.

### Targeting unreleased Spin versions

The above works fine for Spin releases, but we might also want to build components that target pre-release builds of Spin, which may include an ABI and/or API that's different from what ends up in the next release, and yet also different from what was in the previous release.  To handle this case, we propose adding a step to the Spin release process such that the version stored in the `workspace.package.version` field of Spin's Cargo.toml file is changed to `$MAJOR.$MINOR.0-pre0`, where `$MAJOR.$MINOR.0` is the anticipated next Spin release.  Then, each time an ABI or API change is made, the `-pre$N` suffix is incremented.  For example, when Spin 0.9 is released, the version on the main branch is updated to `0.10.0-pre0`, and if an API change is made a few days later, it's updated again to `0.10.0-pre1`.

Using the above scheme, components can target a given pre-release by appending the appropriate `-pre$N` suffix to `spin-sdk-version-$MAJOR-MINOR`, e.g. `spin-sdk-version-0-10-pre1`.

### SDK implementation

The maintainers of each language SDK will need to include source-code level exports for the above functions and update their names as necessary.  `spin-sdk-language-$LANGUAGE` will presumably never change for a given SDK, but `spin-sdk-version-$MAJOR-$MINOR` will need to be updated manually whenever the target Spin version changes.  `spin-sdk-commit-$HASH`, in contrast, should be generated automatically by the build system, e.g. via code generation, preprocessor definition, or similar.  If this is onerous for a given build system, it can simply be omitted.

### Host behavior

When Spin deploys or runs an application, it will have an opportunity to check the versions of its component(s) for compatibility, resulting in one of the following scenarios:

- *No version found*: accept and run the component, raising an error if an incompatibility is detected.  This matches the current behavior.

- *Version found, and it's within the range that this build of Spin supports*: accept and run the component.  Presumably no ABI or API incompatibilities are possible in this scenario, unless the component is reporting an inaccurate version or a breaking SDK change was made without bumping the version.

- *Version found, and it's _not_ within the range that this build of Spin supports*: by default, optimistically accept and run the component, only raising an error if and when an ABI or API incompatibility is detected.  The error message should contain a clear, actionable report based on the component's version.  Alternatively, if "strict" mode is requested (e.g. via a command line option), Spin should reject the application immediately without attempting to deploy or run it, again providing a clear error message.

The rationale for the default, "optimistic" behavior described in the last scenario above is that, although an app may be built against a newer SDK version than Spin supports, it might not use any of the new features of that SDK and thus should run fine on an older Spin (assuming no relevant ABI changes, either).

## Future work

In the above discussion of host behavior, we've left unspecified what range of component versions a given version of Spin supports.  To start with, we can keep it simple: only an exact major and minor version match is supported (but recall that "unsupported" components will also be accepted by default).  Later on, we may expand this to include a range of revisions.  For example, a hypothetical Spin 1.4 might support component versions 1.0-1.4 natively, plus 0.7-0.9 via an emulation layer.

Additionally, that hypothetical Spin 1.4 could provide trapping stub implementations for unknown imports to maximize compatibility with components which target 1.5 and beyond.  This is particularly useful for dynamic languages where dead code elimination is infeasible, and thus a component may import functions that are never actually called.
