mod host_component;

use std::time::Duration;

use anyhow::Result;
use paho_mqtt::Client;
use spin_core::{async_trait, wasmtime::component::Resource};
use spin_world::v1::mqtt as v1;
use spin_world::v2::mqtt::{self as v2, Connection as MqttConnection, Error, Qos};

pub use host_component::OutboundMqttComponent;

pub struct OutboundMqtt {
    allowed_hosts: spin_outbound_networking::AllowedHostsConfig,
    connections: table::Table<Client>,
}

impl Default for OutboundMqtt {
    fn default() -> Self {
        Self {
            allowed_hosts: Default::default(),
            connections: table::Table::new(1024),
        }
    }
}

impl OutboundMqtt {
    fn is_address_allowed(&self, address: &str) -> bool {
        spin_outbound_networking::check_url(address, "mqtt", &self.allowed_hosts)
    }

    async fn establish_connection(
        &mut self,
        address: String,
        keepaliveinterval: Duration,
    ) -> Result<Result<Resource<MqttConnection>, Error>> {
        Ok(async {
            let client = Client::new(address.as_str()).map_err(|_| Error::InvalidAddress)?;
            let conn_opts = paho_mqtt::ConnectOptionsBuilder::new()
                .keep_alive_interval(keepaliveinterval)
                .clean_session(true)
                .finalize();

            client.connect(conn_opts).unwrap();

            self.connections
                .push(client)
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
        keepaliveinterval: u64,
    ) -> Result<Result<Resource<MqttConnection>, Error>> {
        if !self.is_address_allowed(&address) {
            return Ok(Err(Error::InvalidAddress));
        }
        self.establish_connection(address, Duration::from_secs(keepaliveinterval))
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
            let client = self.get_conn(connection).await.map_err(other_error)?;

            // TODO: make QoS parameterised
            let message = paho_mqtt::Message::new(&topic, payload, qos as i32);
            client.publish(message).map_err(other_error)?;
            Ok(())
        }
        .await)
    }

    fn drop(&mut self, connection: Resource<MqttConnection>) -> anyhow::Result<()> {
        self.connections.remove(connection.rep());
        Ok(())
    }
}

fn other_error(e: impl std::fmt::Display) -> Error {
    Error::Other(e.to_string())
}

/// Delegate a function call to the v2::HostConnection implementation
macro_rules! delegate {
    ($self:ident.$name:ident($address:expr, $dur:expr, $($arg:expr),*)) => {{
        if !$self.is_address_allowed(&$address) {
            return Ok(Err(v1::Error::Error));
        }
        let connection = match $self.establish_connection($address, $dur).await? {
            Ok(c) => c,
            Err(_) => return Ok(Err(v1::Error::Error)),
        };
        Ok(<Self as v2::HostConnection>::$name($self, connection, $($arg),*)
            .await?
            .map_err(|_| v1::Error::Error))
    }};
}

#[async_trait]
impl v1::Host for OutboundMqtt {
    async fn publish(
        &mut self,
        address: String,
        topic: String,
        payload: Vec<u8>,
        _qos: v1::Qos,
    ) -> Result<Result<(), v1::Error>> {
        // TODO: map QoS from v1 to v2 or share the enum in WITs.
        delegate!(self.publish(
            address,
            Duration::from_secs(1),
            topic,
            payload,
            Qos::AtLeastOnce
        ))
    }
}

impl OutboundMqtt {
    async fn get_conn(
        &mut self,
        connection: Resource<MqttConnection>,
    ) -> Result<&mut Client, Error> {
        self.connections
            .get_mut(connection.rep())
            .ok_or(Error::Other(
                "could not find connection for resource".into(),
            ))
    }
}
