#![allow(missing_docs)]

wasmtime::component::bindgen!({
    path: "../../wit/preview2",
    async: true,
    interfaces: "
        import config: spin.config
        import postgres: spin.postgres
        import mysql: spin.mysql
        import sqlite: spin.sqlite
        import redis: spin.redis
        import key-value: spin.key-value
        import http: spin.http
    ",
});

wasmtime::component::bindgen!({
    world: "trigger-http",
    path: "../../wit/preview2",
    async: true,
    with: {
        "config": config,
        "http": http,
        "http_types": crate::http_types,
        "key_value": key_value,
        "mysql": mysql,
        "postgres": postgres,
        "rdbms_types": rdbms_types,
        "redis": redis,
        "redis_types": redis_types,
        "sqlite": sqlite,
    },
});

wasmtime::component::bindgen!({
    world: "trigger-redis",
    path: "../../wit/preview2",
    async: true,
    with: {
        "config": config,
        "http": http,
        "http_types": http_types,
        "key_value": key_value,
        "mysql": mysql,
        "postgres": postgres,
        "rdbms_types": rdbms_types,
        "redis": redis,
        "redis_types": crate::redis_types,
        "sqlite": sqlite,
    },
});
