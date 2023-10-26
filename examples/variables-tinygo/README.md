# Spin component in TinyGo using variables

```shell
$ go mod tidy
$ RUST_LOG=spin=trace spin build --up
```

The application can now receive requests on `http://localhost:3000`:

```shell
$ curl -i localhost:3000
HTTP/1.1 200 OK
content-length: 23
date: Tue, 29 Nov 2022 06:59:24 GMT

message:  I'm a teapot
```
