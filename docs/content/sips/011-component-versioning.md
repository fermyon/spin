title = "SIP 011 - Component versioning"
template = "main"
date = "2023-01-24T01:01:01Z"
---

Summary: Embed version information into Spin components for improved compatibility diagnostics.

Owner(s): joel.dice@fermyon.com

Created: Jan 24, 2023

## Background

Currently, Spin has no way to determine which version of SDK the components of an application were built with, which leads to obscure, hard-to-understand error messages for ABI and/or API incompatibilities.  For example, a component built with a Spin 0.4 SDK won't run on Spin 0.7 due to `wit-bindgen` ABI changes, leading to a variety of confusing runtime errors like out-of-bounds memory traps, assertion errors about invalid values, etc.  In addition, new APIs were added between 0.4 and 0.7, and thus a 0.7 component might not run on 0.4 even if the ABI had not changed.  These errors are usually a bit easier to understand (e.g. missing imports), but still don't point clearly to a resolution.  Ideally, the message presented to the user would be something like, "This component was built with a Spin 0.7 SDK and cannot be run using this version of Spin (0.4) -- please use Spin 0.7 or later."

Note that when we say "SDK" in this document, we mean either one of the official Spin SDKs _or_ raw bindings generated from the `WIT` files and the `wit-bindgen` version shipped with a given version of Spin, e.g. for languages which do not yet have an "official" SDK.

## Proposal

Although `WIT` does not yet have a way to express API versions, we can emulate it by introducing a new interface called `spin-version.wit` containing a single function to be exported by the guest component:

```
record version-details {
  commmit-hash: string,
  sdk-language: string
}

spin-version-$MAJOR-$MINOR: func() -> version-details
```

where `$MAJOR` and `$MINOR` are the Spin major and minor version numbers the SDK targets. The intention behind embedding these numbers in the name of the function is that it can be checked statically without instantiating or running the component, and it is immune to ABI changes.  The `version-details` record provides additional details about the SDK used to build the component.  Getting that information requires actually running the component, which may not be possible if the host does not support the ABI for that version, so it won't be useful in all scenarios.  (TODO: should we just encode everything in the name?)

When Spin deploys or runs an application, it will have an opportunity to check the versions of its component(s) for compatibility, resulting in one of the following scenarios:

- *No version found*: accept and run the component, raising an error if an incompatibility is detected.  This matches the current behavior.

- *Version found, and it's within the range that this build of Spin supports*: accept and run the component.  Presumably no ABI or API incompatibilities are possible in this scenario, unless the component is reporting an inaccurate version or a breaking SDK change was made without bumping the version.

- *Version found, and it's _not_ within the range that this build of Spin supports*: by default, optimistically accept and run the component, only raising an error if and when an ABI or API incompatibility is detected.  The error message should contain a clear, actionable report based on the component's version.  Alternatively, if "strict" mode is requested (e.g. via a command line option), Spin should reject the application immediately without attempting to deploy or run it, again providing a clear error message.

The rationale for the default, "optimistic" behavior described in the last scenario above is that, although an app may be built against a newer SDK version than Spin supports, it might not use any of the new features of that SDK and thus should run fine on an older Spin (assuming no relevant ABI changes, either).  In particular, Spin may provide trapping stub implementations for unknown imports to maximize compatibility with applications which may import newer functions but never actually call them.  This is particularly common for dynamic languages where dead code elimination is infeasible.
