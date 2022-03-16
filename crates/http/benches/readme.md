These benchmarks use [criterion.rs](https://github.com/bheisler/criterion.rs); the recommended way to run them is with the [cargo-criterion](https://github.com/bheisler/cargo-criterion) tool:

```sh
$ cargo install cargo-criterion
$ cargo criterion --workspace
```

In order for cargo-criterion to produce nice graphs you need [Graphviz](https://graphviz.org/) installed (available in many system package managers). HTML reports will be written to `target/criterion/reports`