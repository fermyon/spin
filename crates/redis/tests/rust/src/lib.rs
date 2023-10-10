use std::str::{from_utf8, Utf8Error};

wit_bindgen::generate!({
    world: "redis-trigger",
    path: "../../../../wit/preview2",
    exports: {
        "fermyon:spin/inbound-redis@1.0.0": SpinRedis,
    }
});
use exports::fermyon::spin1_0_0::inbound_redis::{self, Error, Payload};

struct SpinRedis;

impl inbound_redis::Guest for SpinRedis {
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
