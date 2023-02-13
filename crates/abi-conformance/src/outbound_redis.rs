use super::Context;
use anyhow::{ensure, Result};
use outbound_redis::{Error, ValueParam, ValueResult};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use wasmtime::{InstancePre, Store};

pub(super) use outbound_redis::add_to_linker;

/// Report of which outbound Redis tests succeeded or failed
#[derive(Serialize)]
pub struct RedisReport {
    /// Result of the Redis `PUBLISH` test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with
    /// \["outbound-redis-publish", "127.0.0.1", "foo", "bar"\] as arguments.  The module should call the
    /// host-implemented `outbound-redis::publish` function with the arguments \["127.0.0.1", "foo", "bar"\] and
    /// expect `ok(unit)` as the result.  The host will assert that said function is called exactly once with the
    /// specified arguments.
    pub publish: Result<(), String>,

    /// Result of the Redis `SET` test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["outbound-redis-set",
    /// "127.0.0.1", "foo", "bar"\] as arguments.  The module should call the host-implemented
    /// `outbound-redis::set` function with the arguments \["127.0.0.1", "foo", "bar"\] and expect `ok(unit)` as
    /// the result.  The host will assert that said function is called exactly once with the specified arguments.
    pub set: Result<(), String>,

    /// Result of the Redis `GET` test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["outbound-redis-get",
    /// "127.0.0.1", "foo"\] as arguments.  The module should call the host-implemented `outbound-redis::get`
    /// function with the arguments \["127.0.0.1", "foo"\] and expect `ok("bar")` (UTF-8-encoded) as the result.
    /// The host will assert that said function is called exactly once with the specified arguments.
    pub get: Result<(), String>,

    /// Result of the Redis `INCR` test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["outbound-redis-incr",
    /// "127.0.0.1", "foo"\] as arguments.  The module should call the host-implemented `outbound-redis::incr`
    /// function with the arguments \["127.0.0.1", "foo"\] and expect `ok(42)` as the result.  The host will assert
    /// that said function is called exactly once with the specified arguments.
    pub incr: Result<(), String>,

    /// Result of the Redis `DEL` test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["outbound-redis-del",
    /// "127.0.0.1", "foo"\] as arguments. The module should call the host-implemented `outbound-redis::del`
    /// function with the arguments \["127.0.0.1", \["foo"\]\] and expect `ok(0)` as the result.  The host will assert
    /// that said function is called exactly once with the specified arguments.
    pub del: Result<(), String>,

    /// Result of the Redis `SADD` test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["outbound-redis-sadd",
    /// "127.0.0.1", "foo", "bar", "baz"\] as arguments. The module should call the host-implemented
    /// `outbound-redis::sadd` function with the arguments \["127.0.0.1", "foo", \["bar", "baz"\]\] and expect
    /// `ok(2)` as the result.  The host will assert that said function is called exactly once with the specified
    /// arguments.
    pub sadd: Result<(), String>,

    /// Result of the Redis `SREM` test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["outbound-redis-srem",
    /// "127.0.0.1", "foo", "bar", "baz"\] as arguments. The module should call the host-implemented
    /// `outbound-redis::srem` function with the arguments \["127.0.0.1", "foo", \["bar", "baz"\]\] and expect
    /// `ok(2)` as the result.  The host will assert that said function is called exactly once with the specified
    /// arguments.
    pub srem: Result<(), String>,

    /// Result of the Redis `SMEMBERS` test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with
    /// \["outbound-redis-smembers", "127.0.0.1", "foo"\] as arguments. The module should call the host-implemented
    /// `outbound-redis::smembers` function with the arguments \["127.0.0.1", "foo"\] and expect `ok(list("bar",
    /// "baz"))` as the result.  The host will assert that said function is called exactly once with the specified
    /// arguments.
    pub smembers: Result<(), String>,

    /// Result of the Redis `execute` test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with
    /// \["outbound-redis-execute", "127.0.0.1", "append", "foo", "baz"\] as arguments. The module should call the
    /// host-implemented `outbound-redis::execute` function with the arguments \["127.0.0.1", "append", "foo",
    /// "baz"\] and expect `ok(list(value::int(3)))` as the result.  The host will assert that said function is
    /// called exactly once with the specified arguments.
    pub execute: Result<(), String>,
}

wit_bindgen_wasmtime::export!("../../wit/ephemeral/outbound-redis.wit");

#[derive(Default)]
pub(super) struct OutboundRedis {
    publish_set: HashSet<(String, String, Vec<u8>)>,
    set_set: HashSet<(String, String, Vec<u8>)>,
    get_map: HashMap<(String, String), Vec<u8>>,
    incr_map: HashMap<(String, String), i64>,
    del_map: HashMap<(String, Vec<String>), i64>,
    sadd_map: HashMap<(String, String, Vec<String>), i64>,
    srem_map: HashMap<(String, String, Vec<String>), i64>,
    smembers_map: HashMap<(String, String), Vec<String>>,
    execute_map: HashMap<(String, String, Vec<String>), Vec<ValueResult>>,
}

impl outbound_redis::OutboundRedis for OutboundRedis {
    fn publish(&mut self, address: &str, channel: &str, payload: &[u8]) -> Result<(), Error> {
        if self
            .publish_set
            .remove(&(address.to_owned(), channel.to_owned(), payload.to_vec()))
        {
            Ok(())
        } else {
            Err(Error::Error)
        }
    }

    fn get(&mut self, address: &str, key: &str) -> Result<Vec<u8>, Error> {
        self.get_map
            .remove(&(address.to_owned(), key.to_owned()))
            .ok_or(Error::Error)
    }

    fn set(&mut self, address: &str, key: &str, value: &[u8]) -> Result<(), Error> {
        if self
            .set_set
            .remove(&(address.to_owned(), key.to_owned(), value.to_vec()))
        {
            Ok(())
        } else {
            Err(Error::Error)
        }
    }

    fn incr(&mut self, address: &str, key: &str) -> Result<i64, Error> {
        self.incr_map
            .remove(&(address.to_owned(), key.to_owned()))
            .map(|value| value + 1)
            .ok_or(Error::Error)
    }

    fn del(&mut self, address: &str, keys: Vec<&str>) -> Result<i64, Error> {
        self.del_map
            .remove(&(
                address.into(),
                keys.into_iter().map(|s| s.to_owned()).collect(),
            ))
            .ok_or(Error::Error)
    }

    fn sadd(&mut self, address: &str, key: &str, values: Vec<&str>) -> Result<i64, Error> {
        self.sadd_map
            .remove(&(
                address.into(),
                key.to_owned(),
                values.into_iter().map(|s| s.to_owned()).collect(),
            ))
            .ok_or(Error::Error)
    }

    fn srem(&mut self, address: &str, key: &str, values: Vec<&str>) -> Result<i64, Error> {
        self.srem_map
            .remove(&(
                address.into(),
                key.to_owned(),
                values.into_iter().map(|s| s.to_owned()).collect(),
            ))
            .ok_or(Error::Error)
    }

    fn smembers(&mut self, address: &str, key: &str) -> Result<Vec<String>, Error> {
        self.smembers_map
            .remove(&(address.into(), key.to_owned()))
            .ok_or(Error::Error)
    }

    fn execute(
        &mut self,
        address: &str,
        command: &str,
        arguments: Vec<ValueParam<'_>>,
    ) -> Result<Vec<ValueResult>, Error> {
        self.execute_map
            .remove(&(
                address.into(),
                command.to_owned(),
                arguments.iter().map(|v| format!("{v:?}")).collect(),
            ))
            .ok_or(Error::Error)
    }
}

pub(super) fn test(store: &mut Store<Context>, pre: &InstancePre<Context>) -> Result<RedisReport> {
    Ok(RedisReport {
        publish: {
            store.data_mut().outbound_redis.publish_set.insert((
                "127.0.0.1".into(),
                "foo".into(),
                "bar".as_bytes().to_vec(),
            ));

            super::run_command(
                store,
                pre,
                &["outbound-redis-publish", "127.0.0.1", "foo", "bar"],
                |store| {
                    ensure!(
                        store.data().outbound_redis.publish_set.is_empty(),
                        "expected module to call `outbound-redis::publish` exactly once"
                    );

                    Ok(())
                },
            )
        },

        set: {
            store.data_mut().outbound_redis.set_set.insert((
                "127.0.0.1".into(),
                "foo".into(),
                "bar".as_bytes().to_vec(),
            ));

            super::run_command(
                store,
                pre,
                &["outbound-redis-set", "127.0.0.1", "foo", "bar"],
                |store| {
                    ensure!(
                        store.data().outbound_redis.set_set.is_empty(),
                        "expected module to call `outbound-redis::set` exactly once"
                    );

                    Ok(())
                },
            )
        },

        get: {
            store.data_mut().outbound_redis.get_map.insert(
                ("127.0.0.1".into(), "foo".into()),
                "bar".as_bytes().to_vec(),
            );

            super::run_command(
                store,
                pre,
                &["outbound-redis-get", "127.0.0.1", "foo"],
                |store| {
                    ensure!(
                        store.data().outbound_redis.get_map.is_empty(),
                        "expected module to call `outbound-redis::get` exactly once"
                    );

                    Ok(())
                },
            )
        },

        incr: {
            store
                .data_mut()
                .outbound_redis
                .incr_map
                .insert(("127.0.0.1".into(), "foo".into()), 41);

            super::run_command(
                store,
                pre,
                &["outbound-redis-incr", "127.0.0.1", "foo"],
                |store| {
                    ensure!(
                        store.data().outbound_redis.incr_map.is_empty(),
                        "expected module to call `outbound-redis::incr` exactly once"
                    );

                    Ok(())
                },
            )
        },

        del: {
            store
                .data_mut()
                .outbound_redis
                .del_map
                .insert(("127.0.0.1".into(), vec!["foo".to_owned()]), 0);

            super::run_command(
                store,
                pre,
                &["outbound-redis-del", "127.0.0.1", "foo"],
                |store| {
                    ensure!(
                        store.data().outbound_redis.del_map.is_empty(),
                        "expected module to call `outbound-redis::del` exactly once"
                    );

                    Ok(())
                },
            )
        },

        sadd: {
            store.data_mut().outbound_redis.sadd_map.insert(
                (
                    "127.0.0.1".into(),
                    "foo".to_owned(),
                    vec!["bar".to_owned(), "baz".to_owned()],
                ),
                0,
            );

            super::run_command(
                store,
                pre,
                &["outbound-redis-del", "127.0.0.1", "foo", "bar", "baz"],
                |store| {
                    ensure!(
                        store.data().outbound_redis.sadd_map.is_empty(),
                        "expected module to call `outbound-redis::sadd` exactly once"
                    );

                    Ok(())
                },
            )
        },

        srem: {
            store.data_mut().outbound_redis.srem_map.insert(
                (
                    "127.0.0.1".into(),
                    "foo".to_owned(),
                    vec!["bar".to_owned(), "baz".to_owned()],
                ),
                0,
            );

            super::run_command(
                store,
                pre,
                &["outbound-redis-del", "127.0.0.1", "foo", "bar", "baz"],
                |store| {
                    ensure!(
                        store.data().outbound_redis.srem_map.is_empty(),
                        "expected module to call `outbound-redis::srem` exactly once"
                    );

                    Ok(())
                },
            )
        },

        smembers: {
            store.data_mut().outbound_redis.smembers_map.insert(
                ("127.0.0.1".into(), "foo".to_owned()),
                vec!["bar".to_owned(), "baz".to_owned()],
            );

            super::run_command(
                store,
                pre,
                &["outbound-redis-del", "127.0.0.1", "foo"],
                |store| {
                    ensure!(
                        store.data().outbound_redis.smembers_map.is_empty(),
                        "expected module to call `outbound-redis::smembers` exactly once"
                    );

                    Ok(())
                },
            )
        },

        execute: {
            store.data_mut().outbound_redis.execute_map.insert(
                (
                    "127.0.0.1".into(),
                    "append".to_owned(),
                    vec!["foo".to_owned(), "baz".to_owned()],
                ),
                vec![ValueResult::Int(3)],
            );

            super::run_command(
                store,
                pre,
                &[
                    "outbound-redis-execute",
                    "127.0.0.1",
                    "append",
                    "foo",
                    "baz",
                ],
                |store| {
                    ensure!(
                        store.data().outbound_redis.execute_map.is_empty(),
                        "expected module to call `outbound-redis::execute` exactly once"
                    );

                    Ok(())
                },
            )
        },
    })
}
