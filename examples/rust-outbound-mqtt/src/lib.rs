use anyhow::Result;
use spin_sdk::{
    http::responses::internal_server_error,
    http::{IntoResponse, Request, Response},
    http_component, mqtt,
};

// The environment variable set in `spin.toml` that points to the
// address of the Mqtt server that the component will publish
// a message to.
const MQTT_ADDRESS_ENV: &str = "MQTT_ADDRESS";

// The environment variable set in `spin.toml` that defines the
// keepalive interval of the Mqtt connection that the component will publish
// a message on.
const MQTT_KEEP_ALIVE_INTERVAL_ENV: &str = "MQTT_KEEP_ALIVE_INTERVAL";

// The environment variable set in `spin.toml` that specifies
// the Mqtt topic that the component will publish to.
const MQTT_TOPIC_ENV: &str = "MQTT_TOPIC";

/// This HTTP component demonstrates fetching a value from Mqtt
/// by key, setting a key with a value, and publishing a message
/// to a Mqtt topic. The component is triggered by an HTTP
/// request served on the route configured in the `spin.toml`.
#[http_component]
fn publish(_req: Request) -> Result<impl IntoResponse> {
    let address = std::env::var(MQTT_ADDRESS_ENV)?;
    let keepaliveinterval = std::env::var(MQTT_KEEP_ALIVE_INTERVAL_ENV)?.parse::<u64>()?;
    let topic = std::env::var(MQTT_TOPIC_ENV)?;
    let message = Vec::from("Eureka!");

    // Publish to Mqtt
    let conn = mqtt::Connection::open(&address, keepaliveinterval)?;

    match conn.publish(&topic, &message, mqtt::Qos::AtLeastOnce) {
        Ok(()) => Ok(Response::new(200, ())),
        Err(_e) => Ok(internal_server_error()),
        // Err(_e) => Ok(Response::new(500, _e.to_string())),
    }
}
