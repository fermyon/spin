use anyhow::Result;
use spin_sdk::{
    http::{Request, Response, responses::internal_server_error},
    mqtt, http_component,
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
/// A simple Spin HTTP component.
#[http_component]
fn hello_world(_req: Request) -> Result<Response> {

    let address = std::env::var(MQTT_ADDRESS_ENV)?;
    let keepaliveinterval = std::env::var(MQTT_KEEP_ALIVE_INTERVAL_ENV)?.parse::<u64>()?;
    let topic = std::env::var(MQTT_TOPIC_ENV)?;
    let message = Vec::from("Eureka!");

    // Publish to Mqtt
    let conn = mqtt::Connection::open(&address, keepaliveinterval)?;

    match conn.publish(&topic, &message, mqtt::Qos::AtLeastOnce) {
        Ok(()) => Ok(Response::builder().status(200).build()),
        Err(_e) => Ok(internal_server_error()),        
    }   
}
