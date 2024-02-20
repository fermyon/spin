use helper::ensure_ok;

const MQTT_ADDRESS_ENV: &str = "MQTT_ADDRESS";

use bindings::fermyon::spin2_0_0::mqtt::{self, Qos};

helper::define_component!(Component);

impl Component {
    fn main() -> Result<(), String> {
        let address = ensure_ok!(std::env::var(MQTT_ADDRESS_ENV));
        let connection = ensure_ok!(mqtt::Connection::open(&address, 10));

        ensure_ok!(connection.publish("spin-example-publish", &b"Eureka!".to_vec(), Qos::AtLeastOnce));

        Ok(())
    }
}
