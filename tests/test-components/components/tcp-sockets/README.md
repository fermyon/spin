# TCP Sockets

Tests the `wasi:sockets` TCP related interfaces

## Expectations

This test component expects the following to be true:
* It is provided the env variable `ADDRESS`
* It has access to a TCP echo server on the address supplied in `ADDRESS`
