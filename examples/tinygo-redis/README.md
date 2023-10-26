# Spin component in TinyGo using the Redis trigger

```shell
$ RUST_LOG=spin=trace spin build --up
```

```shell
$ redis-cli
127.0.0.1:6379> PUBLISH messages test-message
(integer) 1
```
