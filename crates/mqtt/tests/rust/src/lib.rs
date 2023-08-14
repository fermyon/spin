use std::str::{from_utf8, Utf8Error};

wit_bindgen::generate!("mqtt-trigger" in "../../../../wit/preview2");
use exports::fermyon::spin::inbound_mqtt::{self, Error, Payload};

struct SpinMqtt;
export_mqtt_trigger!(SpinMqtt);

impl inbound_mqtt::InboundMqtt for SpinMqtt {
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
