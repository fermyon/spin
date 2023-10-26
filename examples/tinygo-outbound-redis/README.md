# Spin component in TinyGo making an outbound http call to Redis

```shell
$ go mod tidy
$ RUST_LOG=spin=trace spin build --up
```

The application can now receive requests on `http://localhost:3000/publish`:

```shell
$ curl -i localhost:3000/publish
HTTP/1.1 200 OK
content-length: 67
date: Tue, 29 Nov 2022 07:03:52 GMT

mykey value was: myvalue
spin-go-incr value: 1
deleted keys num: 2
```
