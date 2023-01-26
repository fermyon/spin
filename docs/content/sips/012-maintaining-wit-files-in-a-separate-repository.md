# title = "SIP 012 - Maintaining WIT files in a separate repository" template = "main" date = "2023-01-25T01:01:01Z"

Summary: This SIP outlines how to maintain WIT files that Spin (and Spin SDKs) consume outside of the Spin repository.

Owners: michelle@fermyon.com

Created: January 25, 2023

## Background

The WIT files in the `wit/` directory of the current Spin repository define the interfaces that are imported and exported by the Spin SDKs. More specifically, they are referenced by path by the [`wit-bindgen`](https://github.com/bytecodealliance/wit-bindgen) tooling to generate language specific bindings for the SDKs. Example below.

```
// sdk/rust/src/lib.rs
wit_bindgen_rust::import!("../../wit/ephemeral/outbound-redis.wit");
```

The WIT files are also used by multiple crates within the Spin repository. SDKs that live outside the Spin repository are independently maintaining [their own copy](https://github.com/fermyon/spin-dotnet-sdk/tree/main/wit/ephemeral) of the WIT files from the Spin repository. While thinking about moving the Rust and Go SDKs out of the Spin repository ([#1046](https://github.com/fermyon/spin/issues/1046)), it becomes apparent that we'll need to either copy/paste these WIT files into the new SDK repositories (which can be an error prone experience especially when maintaining updates) or find a unified way to reference the same WIT files from different repositories.

## Proposal

Move the WIT files currently housed in the `wit/` directory to a separate repository (https://github.com/fermyon/spin-wit) and use the new repository as a git subtree within any repository (including Spin) that wants to consume and use the WIT files.

### Git subtrees

Git subtrees allow any repository (parent) to pull in another repository (child) into their own repository. The child repository is pulled and copied into a directory in the parent repository with the following command:

`git subtree add --prefix=wit git@github.com:fermyon/spin-wit.git main`

The command above creates a directory called `wit` and pulls the contents of the `fermyon/spin-wit` repository into the `wit` directory. Adopting the `git subtree` workflow would allow the Spin repository to continue to use the wit files as they are being used today and also additionally enable other consumer repositories (SDKs) to pull in the same WIT files into their directory structures. It will also be easier for consumer repositories to keep up with updates to the WIT files as they can use the `git subtree pull` command to pull in new changes.

### Moving WIT files into a new repository

We would want to use tooling like `git filter-branch` to preserve history for the relevant files while migrating to the new repository. The `wit/` directory would then be deleted from the Spin repository and re-added as a git subtree using the `git subtree add` command. These are one time changes.

### Maintaing WIT files over time

#### More on git subtrees

If a parent repository makes a change to a git subtree, that change does not get propogated to the child repository. In order to make changes to the child repository, you have to use the `git subtree push` command. This step can be automated with a github action so that Spin contributors do not have to modify their current workflow for updating WIT files.

#### Workflow for updating WIT files

In order to maintain the current workflow and not have the overhead of managing yet another repository, we'll want to automate updating the child repository (fermyon/spin-wit) from within a github action after a merge to main. The finer details:

1. Someone makes a change to the wit directory from within the Spin repository and opens a pull request to the Spin repository.
2. The pull request is reviewed.
3. The pull request is merged.
4. After being merged, a github action is used to automate updating the `fermyon/spin-wit` repository using `git subtree push`.

The pull request being merged into the Spin repository may or may not contain additional changes outside of the wit directory. This shouldn't matter as the `git subtree push` command will filter out all irrelevant changes. The `fermyon/spin-wit` repository never needs to be modified by anything other than the github action and all consumer repositories can pull in latest updates with the `git subtree pull` command.

## Future design considerations

- Consider flattening the structure of the `wit/` directory by gettind rid of the `wit/emphemeral` directory to more closely conform to the WIT package structure as described by the component model [here](https://github.com/WebAssembly/component-model/pull/141/files#diff-4853dcfce4501ba0f387ca3885f38ac65dc38cc79e4ef16192213d94bce28517R11) and more specifically described [here](https://github.com/WebAssembly/component-model/pull/141/files#diff-4853dcfce4501ba0f387ca3885f38ac65dc38cc79e4ef16192213d94bce28517R188):

```
WIT packages are a flat list of documents, defined in `*.wit` files. The current
thinking for a convention is that projects will have a `wit` folder where all
`wit/*.wit` files within are members of a WIT package."
```

The pull request referenced is currently still open and these changes should be monitored.
