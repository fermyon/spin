use std::str::{from_utf8, Utf8Error};

wit_bindgen::generate!("redis-trigger" in "../../../../wit/preview2");
use exports::fermyon::spin::inbound_redis::{self, Error, Payload};

struct SpinRedis;
export_redis_trigger!(SpinRedis);

impl inbound_redis::InboundRedis for SpinRedis {
    fn handle_message(message: Payload) -> Result<(), Error> {
        println!("Message: {:?}", from_utf8(&message));
        Ok(())
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Self::Error
    }
}
