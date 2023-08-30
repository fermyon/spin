use anyhow::Result;
use spin_sdk::{
    http::{internal_server_error, Request, Response},
    http_component, mqtt,
};

// The environment variable set in `spin.toml` that points to the
// address of the Mqtt server that the component will publish
// a message to.
const MQTT_ADDRESS_ENV: &str = "MQTT_ADDRESS";

// The environment variable set in `spin.toml` that specifies
// the Mqtt topic that the component will publish to.
const MQTT_TOPIC_ENV: &str = "MQTT_TOPIC";

/// This HTTP component demonstrates fetching a value from Mqtt
/// by key, setting a key with a value, and publishing a message
/// to a Mqtt topic. The component is triggered by an HTTP
/// request served on the route configured in the `spin.toml`.
#[http_component]
fn publish(_req: Request) -> Result<Response> {
    let address = std::env::var(MQTT_ADDRESS_ENV)?;
    let topic = std::env::var(MQTT_TOPIC_ENV)?;
    let message = "Hello from Spin!".as_bytes();

    // Publish to Mqtt
    match mqtt::publish(&address, mqtt::Qos::ExactlyOnce, &topic, message) {
        Ok(()) =>  Ok(http::Response::builder().status(200).body(None)?),
        Err(e) => { println!("{e}"); internal_server_error() } ,
    }
}
