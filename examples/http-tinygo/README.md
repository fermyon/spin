# Spin component in TinyGo

```shell
$ go mod tidy
$ RUST_LOG=spin=trace spin build --up
```

The application can now receive requests on `http://localhost:3000`:

```shell
$ curl -i localhost:3000/hello
HTTP/1.1 200 OK
content-type: text/plain
foo: bar
content-length: 440
date: Thu, 26 Oct 2023 18:18:19 GMT

== REQUEST ==
URL:     http://localhost:3000/hello
Method:  GET
Headers:
  "Host": "localhost:3000"
  "User-Agent": "curl/8.1.2"
  "Spin-Full-Url": "http://localhost:3000/hello"
  "Spin-Base-Path": "/"
  "Spin-Client-Addr": "127.0.0.1:52164"
  "Accept": "*/*"
  "Spin-Path-Info": ""
  "Spin-Matched-Route": "/hello"
  "Spin-Raw-Component-Route": "/hello"
  "Spin-Component-Route": "/hello"
Body:
== RESPONSE ==
Hello Fermyon!
```
