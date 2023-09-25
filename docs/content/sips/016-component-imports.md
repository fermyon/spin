title = "SIP 016 - Spin Component Imports" 
template = "main" 
date = "2023-09-22T16:27:43Z"
---

Summary: A proposal to introduce component imports to the Spin 2.0 manifest.

Owner: <brian.hardock@fermyon.com>

Created: Sep 19, 2023

## Background / Goals
The Spin 2.0 framework will support executing [preview2 WebAssembly components](https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md#component-model-explainer). These components will include `import` and `export` statements. This manifest proposal is to enable Spin developers the ability to expressly configure how to satisfy the imports of their components. Furthermore, developers should be enabled to use the exports of a component to satisfy the imports of another.

This SIP is concerned with what's known as "static composition" meaning that a new component binary is produced via composing components together. Static composition here is defined as the process of instantiating components with the exports of other components effectively erasing the imports from the composed component.

## Proposal

### Changes to Spin Manifest 
To support component imports, a new section will be added to the Spin 2.0 manifest design, `[component.<component-id>.import.<import-id>]`. The following sections describe the various options for specifying how to satisfy imports.

> NOTE: The format for each `import` is required to be a kebab-cased name (e.g. `foo-bar`) or interface id (e.g. `foo:bar/baz`) as described by the [Component Model Explainer](https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md#import-and-export-definitions).

### Reference by ID
Imports can be satisfied by associating an import with a component ID which links to a component defined elsewhere in the manifest.

```toml
[component.foobar.import."foo"]
source = "other"

[component.other]
source = { path = "bar.wasm" }
```

### Reference components in a registry
With a future component registry (i.e. `warg`), developers will be able to satisfy their
component's imports using registry references:

```toml
[component.foobar.import."foo"]
source = { registry = "bytecodealliance.org:foobar-dep", version = "1.2.3" }
```

### Satisfy imports using the exports of another component
Below is an example of a component's import declaration. Developers can use the `export` of a component specified by `source` to satisfy the named `import` of the named component.

```toml
[component.foobar.import."foo"]
source = "path/to/component.wasm"
export = "bar"
```

This example demonstrates how to satisfy the `foobar` component's import `foo` using the export named `bar` found in the component at `"path/to/component.wasm"`.

> NOTE: The format for each `export` is required to be a kebab-cased name (e.g. `foo-bar`) or interface id (e.g. `foo:bar/baz`) as described by the [Component Model Explainer](https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md#import-and-export-definitions).

## Future design options
TODO