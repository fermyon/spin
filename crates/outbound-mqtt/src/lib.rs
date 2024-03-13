mod host_component;

use std::time::Duration;

use anyhow::Result;
use rumqttc::AsyncClient;
use spin_core::{async_trait, wasmtime::component::Resource};
use spin_world::v2::mqtt::{self as v2, Connection as MqttConnection, Error, Qos};

pub use host_component::OutboundMqttComponent;

pub struct OutboundMqtt {
    allowed_hosts: spin_outbound_networking::AllowedHostsConfig,
    connections: table::Table<(AsyncClient, rumqttc::EventLoop)>,
}

impl Default for OutboundMqtt {
    fn default() -> Self {
        Self {
            allowed_hosts: Default::default(),
            connections: table::Table::new(1024),
        }
    }
}

const MQTT_CHANNEL_CAP: usize = 1000;

impl OutboundMqtt {
    fn is_address_allowed(&self, address: &str) -> bool {
        spin_outbound_networking::check_url(address, "mqtt", &self.allowed_hosts)
    }

    async fn establish_connection(
        &mut self,
        address: String,
        username: String,
        password: String,
        keep_alive_interval: Duration,
    ) -> Result<Result<Resource<MqttConnection>, Error>> {
        Ok(async {
            let mut conn_opts = rumqttc::MqttOptions::parse_url(address).map_err(|e| {
                tracing::error!("MQTT URL parse error: {e:?}");
                Error::InvalidAddress
            })?;
            conn_opts.set_credentials(username, password);
            conn_opts.set_keep_alive(keep_alive_interval);
            let (client, event_loop) = AsyncClient::new(conn_opts, MQTT_CHANNEL_CAP);

            self.connections
                .push((client, event_loop))
                .map(Resource::new_own)
                .map_err(|_| Error::TooManyConnections)
        }
        .await)
    }
}

impl v2::Host for OutboundMqtt {}

#[async_trait]
impl v2::HostConnection for OutboundMqtt {
    async fn open(
        &mut self,
        address: String,
        username: String,
        password: String,
        keep_alive_interval: u64,
    ) -> Result<Result<Resource<MqttConnection>, Error>> {
        if !self.is_address_allowed(&address) {
            return Ok(Err(v2::Error::ConnectionFailed(format!(
                "address {address} is not permitted"
            ))));
        }
        self.establish_connection(
            address,
            username,
            password,
            Duration::from_secs(keep_alive_interval),
        )
        .await
    }

    async fn publish(
        &mut self,
        connection: Resource<MqttConnection>,
        topic: String,
        payload: Vec<u8>,
        qos: Qos,
    ) -> Result<Result<(), Error>> {
        Ok(async {
            let (client, eventloop) = self.get_conn(connection).await.map_err(other_error)?;
            let qos = convert_to_mqtt_qos_value(qos);

            // Message published to EventLoop (not MQTT Broker)
            client
                .publish_bytes(topic, qos, false, payload.into())
                .await
                .map_err(other_error)?;

            // Poll event loop until outgoing publish event is iterated over to send the message to MQTT broker or capture/throw error.
            // We may revisit this later to manage long running connections and their issues in the connection pool.
            loop {
                let event = eventloop
                    .poll()
                    .await
                    .map_err(|err| v2::Error::ConnectionFailed(err.to_string()))?;

                match event {
                    rumqttc::Event::Outgoing(outgoing_event) => {
                        match outgoing_event {
                            rumqttc::Outgoing::Publish(_) => {
                                return Ok(());
                            }
                            _ => {
                                // We don't care about other outgoing event types in this loop check.
                                continue;
                            }
                        }
                    }
                    rumqttc::Event::Incoming(_) => {
                        // We don't care about incoming event types in this loop check.
                    }
                }
            }
        }
        .await)
    }

    fn drop(&mut self, connection: Resource<MqttConnection>) -> anyhow::Result<()> {
        self.connections.remove(connection.rep());
        Ok(())
    }
}

fn convert_to_mqtt_qos_value(qos: Qos) -> rumqttc::QoS {
    match qos {
        Qos::AtMostOnce => rumqttc::QoS::AtMostOnce,
        Qos::AtLeastOnce => rumqttc::QoS::AtLeastOnce,
        Qos::ExactlyOnce => rumqttc::QoS::ExactlyOnce,
    }
}

fn other_error(e: impl std::fmt::Display) -> Error {
    Error::Other(e.to_string())
}

impl OutboundMqtt {
    async fn get_conn(
        &mut self,
        connection: Resource<MqttConnection>,
    ) -> Result<&mut (AsyncClient, rumqttc::EventLoop), Error> {
        self.connections
            .get_mut(connection.rep())
            .ok_or(Error::Other(
                "could not find connection for resource".into(),
            ))
    }
}
