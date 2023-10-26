# Spin component in TinyGo using the Spin router

```shell
$ go mod tidy
$ RUST_LOG=spin=trace spin build --up
```

The application can now receive requests on `http://localhost:3000`:

```shell
$ curl -i localhost:3000/hello/Fermyon
HTTP/1.1 200 OK
content-length: 16
date: Thu, 26 Oct 2023 18:30:05 GMT

hello, Fermyon!

$ curl -i localhost:3000/this/will/be-special
HTTP/1.1 200 OK
content-length: 24
date: Thu, 26 Oct 2023 18:30:21 GMT

catch all: /be-special!
```
