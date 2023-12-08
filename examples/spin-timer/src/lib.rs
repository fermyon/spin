use std::collections::HashMap;

use clap::Args;
use serde::{Deserialize, Serialize};
use spin_app::MetadataKey;
use spin_core::async_trait;
use spin_trigger::{EitherInstance, TriggerAppEngine, TriggerExecutor};

wasmtime::component::bindgen!({
    path: ".",
    world: "spin-timer",
    async: true
});

pub(crate) type RuntimeData = ();
pub(crate) type _Store = spin_core::Store<RuntimeData>;

#[derive(Args)]
pub struct CliArgs {
    /// If true, run each component once and exit
    #[clap(long)]
    pub test: bool,
}

// The trigger structure with all values processed and ready
pub struct TimerTrigger {
    engine: TriggerAppEngine<Self>,
    speedup: u64,
    component_timings: HashMap<String, u64>,
}

// Application settings (raw serialization format)
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TriggerMetadata {
    r#type: String,
    speedup: Option<u64>,
}

// Per-component settings (raw serialization format)
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimerTriggerConfig {
    component: String,
    interval_secs: u64,
}

const TRIGGER_METADATA_KEY: MetadataKey<TriggerMetadata> = MetadataKey::new("trigger");

#[async_trait]
impl TriggerExecutor for TimerTrigger {
    const TRIGGER_TYPE: &'static str = "timer";

    type RuntimeData = RuntimeData;

    type TriggerConfig = TimerTriggerConfig;

    type RunConfig = CliArgs;

    async fn new(engine: spin_trigger::TriggerAppEngine<Self>) -> anyhow::Result<Self> {
        let speedup = engine
            .app()
            .require_metadata(TRIGGER_METADATA_KEY)?
            .speedup
            .unwrap_or(1);

        let component_timings = engine
            .trigger_configs()
            .map(|(_, config)| (config.component.clone(), config.interval_secs))
            .collect();

        Ok(Self {
            engine,
            speedup,
            component_timings,
        })
    }

    async fn run(self, config: Self::RunConfig) -> anyhow::Result<()> {
        if config.test {
            for component in self.component_timings.keys() {
                self.handle_timer_event(component).await?;
            }
        } else {
            // This trigger spawns threads, which Ctrl+C does not kill.  So
            // for this case we need to detect Ctrl+C and shut those threads
            // down.  For simplicity, we do this by terminating the process.
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.unwrap();
                std::process::exit(0);
            });

            let speedup = self.speedup;
            tokio_scoped::scope(|scope| {
                // For each component, run its own timer loop
                for (c, d) in &self.component_timings {
                    scope.spawn(async {
                        let duration = tokio::time::Duration::from_millis(*d * 1000 / speedup);
                        loop {
                            tokio::time::sleep(duration).await;
                            self.handle_timer_event(c).await.unwrap();
                        }
                    });
                }
            });
        }
        Ok(())
    }
}

impl TimerTrigger {
    async fn handle_timer_event(&self, component_id: &str) -> anyhow::Result<()> {
        // Load the guest...
        let (instance, mut store) = self.engine.prepare_instance(component_id).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };
        let instance = SpinTimer::new(&mut store, &instance)?;
        // ...and call the entry point
        instance.call_handle_timer_request(&mut store).await
    }
}
