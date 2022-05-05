# Spin HTTP handler in (Tiny)Go

To build:

1. Change the module redirect to point to your local Spin repository, e.g.

```
replace github.com/fermyon/spin/sdk/go v0.0.0 => ../spin/sdk/go/
#        path from here to your checkout of Spin ^^^^^^^
```

2. Compile

```shell
$ spin build
```

To run after building

```shell
spin up
```
