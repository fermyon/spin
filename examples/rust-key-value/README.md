# Spin Key Value component in Rust

```shell
$ RUST_LOG=spin=trace spin build --up
```

The application can now receive requests on `http://localhost:3000`:

```shell
$ curl -i -X POST -d "ok!" localhost:3000/test
HTTP/1.1 200 OK
content-length: 0
date: Tue, 25 Apr 2023 14:25:43 GMT

$ curl -i -X GET localhost:3000/test
HTTP/1.1 200 OK
content-length: 3
date: Tue, 25 Apr 2023 14:25:54 GMT

ok!

$ curl -i -X DELETE localhost:3000/test
HTTP/1.1 200 OK
content-length: 0
date: Tue, 25 Apr 2023 14:26:30 GMT

$ curl -i -X GET localhost:3000/test
HTTP/1.1 404 Not Found
content-length: 0
date: Tue, 25 Apr 2023 14:31:53 GMT
```
