## Automatically deserializing JSON request bodies in Rust HTTP

This sample shows using the `http::Request<Json<T>>` request type to accept JSON and automatically deserialize it into a Rust struct that implements `serde::Deserialize`.

To test it, run `spin up --build` and then POST to localhost:3000 e.g.:

```
curl -X POST -d '{"name": "Vyvyan"}' localhost:3000
```
