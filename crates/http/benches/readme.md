These benchmarks use [criterion.rs](https://github.com/bheisler/criterion.rs); the recommended way to run them is with the [cargo-criterion](https://github.com/bheisler/cargo-criterion) tool:

```sh
$ cargo install cargo-criterion
$ cargo criterion --workspace
```

HTML reports will be written to `target/criterion/reports`