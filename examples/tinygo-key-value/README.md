# Spin component in TinyGo

```shell
$ go mod tidy
$ RUST_LOG=spin=trace spin build --up
```

The application can now receive requests on `http://localhost:3000/test`:

```shell
$ curl -i localhost:3000/test
HTTP/1.1 200 OK
content-length: 67
date: Tue, 29 Nov 2022 07:03:52 GMT
```
