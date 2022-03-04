use spin_redis_trigger::{Error, Payload};
use std::str::{from_utf8, Utf8Error};

wit_bindgen_rust::export!("../../../../wit/ephemeral/spin-redis-trigger.wit");

struct SpinRedisTrigger {}

impl spin_redis_trigger::SpinRedisTrigger for SpinRedisTrigger {
    fn handler(payload: Payload) -> Result<(), Error> {
        println!("Message: {:?}", from_utf8(&payload));
        Ok(())
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Self::Error
    }
}
