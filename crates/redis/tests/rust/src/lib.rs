use std::str::{from_utf8, Utf8Error};

wit_bindgen::generate!("spin-redis" in "../../../../sdk/rust/macro/wit");

use inbound_redis::{Error, Payload};

struct SpinRedis;
export_spin_redis!(SpinRedis);
#[export_name = "spin-sdk-version-1-2-pre0"]
extern "C" fn __spin_sdk_version() {}

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
