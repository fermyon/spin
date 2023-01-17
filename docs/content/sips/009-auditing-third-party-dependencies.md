title = "SIP 009 - Auditing third party dependencies using `cargo vet`"
template = "main"
date = "2023-01-05T01:01:01Z"

---

## Background

The use of third party dependencies in modern software development is essential.
This is true for language ecosystems such as JavaScript, Python, or Rust,
where projects written in these languages heavily depend on packages from NPM,
PyPi, and crates.io.

Relying on centralized repositories for these dependencies can lead to the
introduction of vulnerabilities in projects, either accidentally, or as a
result of malicious actors.
While automated scanners and vulnerability detectors are useful, the integrity
of the project relies on the maintainers, who are responsible for the effects
these dependencies have. As a result, human audits are extremely useful,
particularly when combined with automated scanners.

While the Spin project is using automated scanners such as [GitHub's automated
dependency graph](https://github.com/fermyon/spin/network/dependencies) and
[Dependabot](https://github.com/features/security), it currently lacks a
standardized framework for human audits of dependencies.

The Spin project is built using the Rust language and ecosystem, which has
an emerging project designed to audit the packages used by a project,
[`cargo vet`](https://mozilla.github.io/cargo-vet/index.html).
The project aims to tackle this issue in a way that is unobtrusive,
allows the maintainers to gradually audit more of their dependencies
without placing an initial burden, and encourages the community to
share existing audits from trusted organizations.

Specifically, when run, `cargo vet` will verify all dependencies of a project
against the audits performed by the maintainers, or by organizations they trust.
If a dependency has not been audited yet, the tool provides assistance in
performing the audit.

## Proposal

This SIP proposes that the Spin project should adopt the `cargo vet` project
to aide with the process of reviewing dependencies that are used by the project.

Since the Wasmtime project is already using `cargo vet` and [is publishing the audits
for its dependencies](https://github.com/bytecodealliance/wasmtime/blob/main/supply-chain/audits.toml),
and since Wasmtime is one of Spin's primary dependencies, the Spin project can
benefit from the audits performed by the Bytecode Alliance, giving the project
a trusted source for audits.
Together with using `cargo vet`, this SIP proposes importing the audit results
from [Wasmtime](https://github.com/bytecodealliance/wasmtime/blob/main/supply-chain/audits.toml)
and [Mozilla](https://hg.mozilla.org/mozilla-central/raw-file/tip/supply-chain/audits.toml).

### What is the initial state of the `supply-chain` directory?

Initially, all dependencies that are not already audited by a trusted organization
will be added as exemptions - which effectively marks the starting point of the
auditing process as trusted. This is to avoid the potentially massive initial effort
required to validate _all_ transitive dependencies. However, the first time a dependency
would get updated, `cargo vet` will require explicitly auditing the new version,
or manually adding it as an exemption.

### How is an audit performed?

[Performing audits](https://mozilla.github.io/cargo-vet/performing-audits.html)
can be the result of two events:

- manually removing a dependency from the exemptions list
- a dependency has been updated

When that happens, `cargo vet` will fail:

```bash
$ cargo vet
  Vetting Failed!

  3 unvetted dependencies:
      bar:1.5 missing ["safe-to-deploy"]
      baz:1.3 missing ["safe-to-deploy"]
      foo:1.2.1 missing ["safe-to-deploy"]

  recommended audits for safe-to-deploy:
      cargo vet diff foo 1.2 1.2.1  (10 lines)
      cargo vet diff bar 2.1.1 1.5  (253 lines)
      cargo vet inspect baz 1.3     (2033 lines)

  estimated audit backlog: 2296 lines

  Use |cargo vet certify| to record the audits.
```

[From the documentation](https://mozilla.github.io/cargo-vet/performing-audits.html):

> Note that if other versions of a given crate have already been verified,
there will be multiple ways to perform the review: either from scratch, or
relative to one or more already-audited versions. In these cases, `cargo vet`
computes all the possible approaches and selects the smallest one.
>
> You can, of course, choose to add one or more unvetted dependencies to the
exemptions list instead of auditing them. This may be expedient in some situations,
though doing so frequently undermines the value provided by the tool.

Once a package is chosen for an audit, `cargo vet inspect` guides the user
through performing the audit:

```bash
$ cargo vet inspect baz 1.3
You are about to inspect version 1.3 of 'baz', likely to certify it for "safe-to-deploy", which means:
   ...
You can inspect the crate here: https://sourcegraph.com/crates/baz@v1.3

(press ENTER to open in your browser, or re-run with --mode=local)

$ cargo vet certify baz 1.3

  I, Alice, certify that I have audited version 1.3 of baz in accordance with
  the following criteria:

  ...

 (type "yes" to certify): yes

  Recorded full audit of baz version 1.3
```

Similarly, `cargo vet diff` will help guide the user through reviewing the diff
between two versions and certifying the audit.

Finally, `cargo vet suggest` is the command whose goal is to guide maintainers through
[shrinking the exemptions list](https://mozilla.github.io/cargo-vet/performing-audits.html?highlight=exemption#shrinking-the-exemptions-table).

### What are the audit criteria?

By default, `cargo vet` suggests two [built-in criteria for audits](https://mozilla.github.io/cargo-vet/built-in-criteria.html):

- `safe-to-run` - this could be thought of as "this development dependency is
safe to run on local workstations and CI"
- `safe-to-deploy` - this could be thought of as "this dependency is safe to
run in production environments", and implies `safe-to-run`.

### Alternatives

The main alternative seems to be [Crev](https://github.com/crev-dev/crev/),
which does have a `cargo crev` command. However, there are two main reasons
why `cargo vet` would be a better alternative:

- Crev seems to be a far more complex system compared to `cargo vet`
- for the Spin project, trusting the Bytecode Alliance audits for the
Wasmtime project reduces the burden on the Spin project maintainers significantly.
