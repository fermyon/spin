title = "Contributing to Spin"
template = "main"
date = "2022-03-14T00:22:56Z"
[extra]
url = "https://github.com/fermyon/spin/blob/main/docs/content/contributing.md"
---

We are delighted that you are interested in making Spin better! Thank you! This
document will guide you in making your first contribution to the project.

First, any contribution and interaction on any Fermyon project MUST follow our
[code of conduct](https://www.fermyon.com/code-of-conduct). Thank you for being
part of an inclusive and open community!
We welcome and appreciate contributions of all types — opening issues, fixing
typos, adding examples, one-liner code fixes, tests, or complete features.

If you plan on contributing anything complex, please go through the issue and PR
queues first to make sure someone else has not started working on it. If it
doesn't exist already, please open an issue so you have a chance to get feedback
from the community and the maintainers before you start working on your feature.

## Making code contributions to Spin

The following guide is intended to make sure your contribution can get merged as
soon as possible. First, make sure you have the following prerequisites
configured:

- [Rust](https://www.rust-lang.org/) at
  [1.56+](https://www.rust-lang.org/tools/install) with the `wasm32-wasi` and
  `wasm32-unknown-unknown` targets configured
  (`rustup target add wasm32-wasi && rustup target add wasm32-unknown-unknown`)
- [`rustfmt`](https://github.com/rust-lang/rustfmt) and
  [`clippy`](https://github.com/rust-lang/rust-clippy) configured for your Rust
  installation
- `make`
- [Bindle server v0.8.0](https://github.com/deislabs/bindle/releases/tag/v0.8.0)
  in your system path.
- if you are a VS Code user, we recommend the
  [`rust-analyzer`](https://rust-analyzer.github.io/) and
  [`autobindle`](https://github.com/fermyon/autobindle) extensions.
- please ensure you
  [configure adding a GPG signature to your commits](https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification)
  as well as appending a sign-off message (`git commit -S -s`)

Once you have set up the prerequisites and identified the contribution you want
to make to Spin, make sure you can correctly build the project:

```
# clone the repository
$ git clone https://github.com/fermyon/spin && cd spin
# add a new remote pointing to your fork of the project
$ git remote add fork https://github.com/<your-username>/spin
# create a new branch for your work
$ git checkout -b <your-branch>

# if you are making a documentation contribution,
# you can skip compiling and running the tests.

# build a release version of the Spin CLI
$ cargo build --release
# make sure compilation is successful
$ ./target/release/spin --help

# run the tests and make sure they pass
$ make test
```

Now you should be ready to start making your contribution. To familiarize
yourself with the Spin project, please read the
[document about extending Spin](/extending-and-embedding). Since most of Spin is implemented in
Rust, we try to follow the common Rust coding conventions (keep an eye on the
recommendations from Clippy!) If applicable, add unit or integration tests to
ensure your contribution is correct.

Build the project and run the tests (`make build test`), and if everything is
successful, you should be ready to commit your changes. We try to follow the
[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
guidelines for writing commit messages:

```shell
$ git commit -S -s -m "<your commit message that follows https://www.conventionalcommits.org/en/v1.0.0/>"
```

We try to only keep useful changes as separate commits — if you prefer to commit
often, please
[cleanup the commit history](https://git-scm.com/book/en/v2/Git-Tools-Rewriting-History)
before opening a pull request. Once you are happy with your changes you can push
the branch to your fork:

```shell
# "fork" is the name of the git remote pointing to your fork
$ git push fork
```

Now you are ready to create a pull request. Thank you for your contribution!
