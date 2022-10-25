use super::Context;
use anyhow::{ensure, Result};
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
}

wit_bindgen_wasmtime::export!("../../wit/ephemeral/outbound-redis.wit");

#[derive(Default)]
pub(super) struct OutboundRedis {
    publish_set: HashSet<(String, String, Vec<u8>)>,
    set_set: HashSet<(String, String, Vec<u8>)>,
    get_map: HashMap<(String, String), Vec<u8>>,
    incr_map: HashMap<(String, String), i64>,
    del_map: HashMap<(String, String), i64>,
}

impl outbound_redis::OutboundRedis for OutboundRedis {
    fn publish(
        &mut self,
        address: &str,
        channel: &str,
        payload: &[u8],
    ) -> Result<(), outbound_redis::Error> {
        if self
            .publish_set
            .remove(&(address.to_owned(), channel.to_owned(), payload.to_vec()))
        {
            Ok(())
        } else {
            Err(outbound_redis::Error::Error)
        }
    }

    fn get(&mut self, address: &str, key: &str) -> Result<Vec<u8>, outbound_redis::Error> {
        self.get_map
            .remove(&(address.to_owned(), key.to_owned()))
            .ok_or(outbound_redis::Error::Error)
    }

    fn set(&mut self, address: &str, key: &str, value: &[u8]) -> Result<(), outbound_redis::Error> {
        if self
            .set_set
            .remove(&(address.to_owned(), key.to_owned(), value.to_vec()))
        {
            Ok(())
        } else {
            Err(outbound_redis::Error::Error)
        }
    }

    fn incr(&mut self, address: &str, key: &str) -> Result<i64, outbound_redis::Error> {
        self.incr_map
            .remove(&(address.to_owned(), key.to_owned()))
            .map(|value| value + 1)
            .ok_or(outbound_redis::Error::Error)
    }

    fn del(&mut self, address: &str, keys: Vec<&str>) -> Result<i64, outbound_redis::Error> {
        self.del_map
            .remove(&(address.into(), format!("{keys:?}")))
            .ok_or(outbound_redis::Error::Error)
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
            store.data_mut().outbound_redis.del_map.insert(
                ("127.0.0.1".into(), format!("{:?}", vec!["foo".to_owned()])),
                0,
            );

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
    })
}
