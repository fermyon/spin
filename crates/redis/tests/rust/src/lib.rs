use std::str::{from_utf8, Utf8Error};

wit_bindgen::generate!("spin" in "../../../../wit/ephemeral");

use inbound_redis::{Error, Payload};

struct SpinRedis;
export_spin!(SpinRedis);

impl inbound_redis::InboundRedis for SpinRedis {
    fn handle_message(message: Payload) -> Result<(), Error> {
        println!("Message: {:?}", from_utf8(&message));
        Ok(())
    }
}

impl inbound_http::InboundHttp for SpinRedis {
    fn handle_request(_req: inbound_http::Request) -> inbound_http::Response {
        todo!()
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Self::Error
    }
}
