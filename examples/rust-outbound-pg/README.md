# Spin Outbound PostgreSQL example

This example shows how to access a PostgreSQL database from Spin component.

## Spin up

From example root:

```
createdb spin_dev
psql -d spin_dev -f db/testdata.sql
RUST_LOG=spin=trace spin build --up
```

Curl the read route:

```
$ curl -i localhost:3000/read
HTTP/1.1 200 OK
content-length: 501
date: Sun, 25 Sep 2022 15:45:02 GMT

Found 2 article(s) as follows:
article: Article {
    id: 1,
    title: "My Life as a Goat",
    content: "I went to Nepal to live as a goat, and it was much better than being a butler.",
    authorname: "E. Blackadder",
}
article: Article {
    id: 2,
    title: "Magnificent Octopus",
    content: "Once upon a time there was a lovely little sausage.",
    authorname: "S. Baldrick",
}

(Column info: id:DbDataType::Int32, title:DbDataType::Str, content:DbDataType::Str, authorname:DbDataType::Str)
```

Curl the write route:

```
$ curl -i localhost:3000/write
HTTP/1.1 200 OK
content-length: 9
date: Sun, 25 Sep 2022 15:46:22 GMT

Count: 3
```
