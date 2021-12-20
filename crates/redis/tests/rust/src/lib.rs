use std::str::{from_utf8, Utf8Error};

use spin_redis_trigger_v01::*;

wit_bindgen_rust::export!("../../wit/spin_redis_trigger_v01.wit");

struct SpinRedisTriggerV01 {}

impl spin_redis_trigger_v01::SpinRedisTriggerV01 for SpinRedisTriggerV01 {
    fn handler(payload: Payload) -> Result<(), Error> {
        println!("Message: {}", from_utf8(&payload)?);
        Ok(())
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Self::Error
    }
}
