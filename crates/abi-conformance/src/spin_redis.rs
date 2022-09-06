use super::Context;
use spin_redis::SpinRedis;
use std::{error, fmt};
use wasmtime::{InstancePre, Store};

pub use spin_redis::SpinRedisData;

wit_bindgen_wasmtime::import!("../../wit/ephemeral/spin-redis.wit");

impl fmt::Display for spin_redis::Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Success => f.write_str("redis success"),
            Self::Error => f.write_str("redis error"),
        }
    }
}

impl error::Error for spin_redis::Error {}

pub(super) fn test(store: &mut Store<Context>, pre: &InstancePre<Context>) -> Result<(), String> {
    super::run(|| {
        let instance = &pre.instantiate(&mut *store)?;
        let handle = SpinRedis::new(&mut *store, instance, |context| &mut context.spin_redis)?;
        handle.handle_redis_message(store, b"Hello, SpinRedis!")??;

        Ok(())
    })
}
