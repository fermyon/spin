mod host_component;

use std::{
    collections::{hash_map::Entry, HashMap},
    time::Duration,
};

use anyhow::Result;
use paho_mqtt::Client;
use spin_core::async_trait;
use spin_world::{mqtt as outbound_mqtt, mqtt_types::Error};

pub use host_component::OutboundMqttComponent;

#[derive(Default)]
pub struct OutboundMqtt {
    connections: HashMap<String, Client>,
}

#[async_trait]
impl outbound_mqtt::Host for OutboundMqtt {
    async fn publish(
        &mut self,
        address: String,
        topic: String,
        payload: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        Ok(async {
            let client = self.get_conn(&address).await.map_err(log_error)?;
            let message = paho_mqtt::Message::new(&topic, payload, 0);
            client.publish(message).map_err(log_error)?;
            Ok(())
        }
        .await)
    }
}

impl OutboundMqtt {
    async fn get_conn(&mut self, address: &str) -> Result<&mut Client> {
        let client = match self.connections.entry(address.to_string()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let client = Client::new(address.to_string())?;

                let conn_opts = paho_mqtt::ConnectOptionsBuilder::new()
                    .keep_alive_interval(Duration::from_secs(60))
                    .clean_session(true)
                    .finalize();
                client.connect(conn_opts)?;

                v.insert(client)
            }
        };
        Ok(client)
    }
}

fn log_error(err: impl std::fmt::Debug) -> Error {
    tracing::warn!("Outbound Mqtt error: {err:?}");
    Error::Error
}
