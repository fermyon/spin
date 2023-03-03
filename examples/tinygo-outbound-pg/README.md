# Spin component with outbound postgres in TinyGo

```shell
$ RUST_LOG=spin=trace spin build --up
```

The application can now receive requests on `http://127.0.0.1:3000`:

```shell
$ curl -i 127.0.0.1:3000
HTTP/1.1 200 OK
content-length: 1295
date: Fri, 03 Mar 2023 16:41:06 GMT

Inserted rows=0
Columns = id, title, content, authorname, coauthor,
{"id": "1","title": "My Life as a Goat","content": "I went to Nepal to live as a goat, and it was much better than being a butler.","authorname": "E. Blackadder","coauthor": ""}{"id": "2","title": "Magnificent Octopus","content": "Once upon a time there was a lovely little sausage.","authorname": "S. Baldrick","coauthor": ""}{"id": "3","title": "Test Article","content": "This article was inserted by the example module","authorname": "spin","coauthor": "tingyo-outbound-pg"}{"id": "4","title": "Test Article","content": "This article was inserted by the example module","authorname": "spin","coauthor": "tingyo-outbound-pg"}{"id": "5","title": "Test Article","content": "This article was inserted by the example module","authorname": "spin","coauthor": "tingyo-outbound-pg"}{"id": "6","title": "Test Article","content": "This article was inserted by the example module","authorname": "spin","coauthor": "tingyo-outbound-pg"}{"id": "7","title": "Test Article","content": "This article was inserted by the example module","authorname": "spin","coauthor": "tingyo-outbound-pg"}{"id": "8","title": "Test Article","content": "This article was inserted by the example module","authorname": "spin","coauthor": "tingyo-outbound-pg"}
```
