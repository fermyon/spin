mod host;

use std::sync::Arc;
use std::time::Duration;

use host::other_error;
use host::InstanceState;
use rumqttc::{AsyncClient, Event, Incoming, Outgoing, QoS};
use spin_core::async_trait;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factors::{
    ConfigureAppContext, Factor, InstanceBuilders, PrepareContext, RuntimeFactors,
    SelfInstanceBuilder,
};
use spin_world::v2::mqtt::{self as v2, Error, Qos};
use tokio::sync::Mutex;

pub use host::{ClientCreator, MqttClient};

pub struct OutboundMqttFactor {
    create_client: Arc<dyn ClientCreator>,
}

impl OutboundMqttFactor {
    pub fn new(create_client: Arc<dyn ClientCreator>) -> Self {
        Self { create_client }
    }
}

impl Factor for OutboundMqttFactor {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::v2::mqtt::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        _ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        Ok(())
    }

    fn prepare<T: RuntimeFactors>(
        &self,
        _ctx: PrepareContext<Self>,
        builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let allowed_hosts = builders
            .get_mut::<OutboundNetworkingFactor>()?
            .allowed_hosts();
        Ok(InstanceState::new(
            allowed_hosts,
            self.create_client.clone(),
        ))
    }
}

impl SelfInstanceBuilder for InstanceState {}

// This is a concrete implementation of the MQTT client using rumqttc.
pub struct NetworkedMqttClient {
    inner: rumqttc::AsyncClient,
    event_loop: Mutex<rumqttc::EventLoop>,
}

const MQTT_CHANNEL_CAP: usize = 1000;

impl NetworkedMqttClient {
    pub fn create(
        address: String,
        username: String,
        password: String,
        keep_alive_interval: Duration,
    ) -> Result<Self, Error> {
        let mut conn_opts = rumqttc::MqttOptions::parse_url(address).map_err(|e| {
            tracing::error!("MQTT URL parse error: {e:?}");
            Error::InvalidAddress
        })?;
        conn_opts.set_credentials(username, password);
        conn_opts.set_keep_alive(keep_alive_interval);
        let (client, event_loop) = AsyncClient::new(conn_opts, MQTT_CHANNEL_CAP);
        Ok(Self {
            inner: client,
            event_loop: Mutex::new(event_loop),
        })
    }
}

#[async_trait]
impl MqttClient for NetworkedMqttClient {
    async fn publish_bytes(&self, topic: String, qos: Qos, payload: Vec<u8>) -> Result<(), Error> {
        let qos = match qos {
            Qos::AtMostOnce => rumqttc::QoS::AtMostOnce,
            Qos::AtLeastOnce => rumqttc::QoS::AtLeastOnce,
            Qos::ExactlyOnce => rumqttc::QoS::ExactlyOnce,
        };
        // Message published to EventLoop (not MQTT Broker)
        self.inner
            .publish_bytes(topic, qos, false, payload.into())
            .await
            .map_err(other_error)?;

        // Poll event loop until outgoing publish event is iterated over to send the message to MQTT broker or capture/throw error.
        // We may revisit this later to manage long running connections, high throughput use cases and their issues in the connection pool.
        let mut lock = self.event_loop.lock().await;
        loop {
            let event = lock
                .poll()
                .await
                .map_err(|err| v2::Error::ConnectionFailed(err.to_string()))?;

            match (qos, event) {
                (QoS::AtMostOnce, Event::Outgoing(Outgoing::Publish(_)))
                | (QoS::AtLeastOnce, Event::Incoming(Incoming::PubAck(_)))
                | (QoS::ExactlyOnce, Event::Incoming(Incoming::PubComp(_))) => break,

                (_, _) => continue,
            }
        }
        Ok(())
    }
}
