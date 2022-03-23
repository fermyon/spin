use spin_redis::{Error, Payload};
use std::str::{from_utf8, Utf8Error};

wit_bindgen_rust::export!("../../../../wit/ephemeral/spin-redis.wit");

struct SpinRedis {}

impl spin_redis::SpinRedis for SpinRedis {
    fn handle_redis_message(message: Payload) -> Result<(), Error> {
        println!("Message: {:?}", from_utf8(&message));
        Ok(())
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Self::Error
    }
}
