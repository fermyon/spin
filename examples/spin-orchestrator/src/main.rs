// Example of a domain specific wasm orchestrator plugin.
// Orchestration logic is defined in the orchestrator plugin.
use std::collections::HashMap;

use anyhow::{Error, Ok};
use clap::{Args, Parser};
use serde::{Deserialize, Serialize};
use spin_app::MetadataKey;
use spin_core::async_trait;
use spin_trigger::{
    cli::TriggerExecutorCommand, EitherInstance, TriggerAppEngine, TriggerExecutor,
};

// Module 1 WIT bindings
wasmtime::component::bindgen!({
    path: "spin-orchestrator-module1.wit",
    world: "spin-orchestrator-module1",
    async: true
});

// Module 2 WIT bindings
wasmtime::component::bindgen!({
    path: "spin-orchestrator-module2.wit",
    world: "spin-orchestrator-module2",
    async: true
});

pub(crate) type RuntimeData = ();
// pub(crate) type _Store = spin_core::Store<RuntimeData>;

type Command = TriggerExecutorCommand<OrchestratorTrigger>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let t = Command::parse();
    t.run().await
}

#[derive(Args)]
pub struct CliArgs {
    /// If true, run each component once and exit
    #[clap(long)]
    pub test: bool,
}

// The orchestrator structure with all values processed and ready
struct OrchestratorTrigger {
    engine: TriggerAppEngine<Self>,
    retry_policy_enabled: RetryPolicyEnabled,
    component_timings: HashMap<String, String>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
enum RetryPolicyEnabled {    
    None,
    Some(bool),
}

impl Default for RetryPolicyEnabled {
    fn default() -> Self {
        RetryPolicyEnabled::Some(false)
    }
}

// Application settings (raw serialization format)
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TriggerMetadata {
    r#type: String,
    retry_policy_enabled: RetryPolicyEnabled,
}

// Per-component settings (raw serialization format)
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]

pub struct ModuleTriggerConfig {
    component: String,
    config: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Module1TriggerConfig {    
    ssl_only: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Module2TriggerConfig {    
    batch_size: u64,
}

const TRIGGER_METADATA_KEY: MetadataKey<TriggerMetadata> = MetadataKey::new("trigger");

#[async_trait]
impl TriggerExecutor for OrchestratorTrigger {
    const TRIGGER_TYPE: &'static str = "orchestrator";

    type RuntimeData = RuntimeData;

    type TriggerConfig = ModuleTriggerConfig;
    
    type RunConfig = CliArgs;

    async fn new(engine: spin_trigger::TriggerAppEngine<Self>) -> anyhow::Result<Self> {
        let retry_policy_enabled = engine
            .app()
            .require_metadata(TRIGGER_METADATA_KEY)?
            .retry_policy_enabled;

        let component_timings: HashMap<String, String> = engine
            .trigger_configs()
            .map(|(_, config)| (config.component.clone(), config.config.clone()))
            .collect();

        Ok(Self {
            engine,
            retry_policy_enabled,
            component_timings,
        })
    }

    async fn run(self, config: Self::RunConfig) -> anyhow::Result<()> {
        if config.test {
            for component in self.component_timings.keys() {
                self.handle_module1_init_event(component).await?;
                self.handle_module2_init_event(component).await?;
            }
        } else {
            // This trigger spawns threads, which Ctrl+C does not kill.  So
            // for this case we need to detect Ctrl+C and shut those threads
            // down.  For simplicity, we do this by terminating the process.
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.unwrap();
                std::process::exit(0);
            });

            // Check if retry policy is enabled for trasient errors.
            let _retry_policy_enabled = self.retry_policy_enabled;

            tokio_scoped::scope(|scope| {

                // For each component, run its own timer loop
                for (c, d) in &self.component_timings {
                    match c.as_str() {
                        "module1" => {
                            let config: Module1TriggerConfig = serde_json::from_str(d).unwrap();
                            println!("{}: Config: {:?}", c, config);
                            scope.spawn(async {
                                self.handle_module1_init_event(c).await.unwrap();
                            });
                        },
                        "module2" => {
                            let config: Module2TriggerConfig = serde_json::from_str(d).unwrap();
                            println!("{}: Config: {:?}", c, config);
                            scope.spawn(async {
                                self.handle_module2_init_event(c).await.unwrap();
                            });
                        },
                        _ => {
                            panic!("Unknown module Id {}: Config: {:?}", c, d);
                        }
                    }                                        
                }
            });
        }
        Ok(())
    }
}

impl OrchestratorTrigger {
    async fn handle_module1_init_event(&self, component_id: &str) -> anyhow::Result<String> {
        // Load the guest...
        let (instance, mut store) = self.engine.prepare_instance(component_id).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };

        let instance = SpinOrchestratorModule1::new(&mut store, &instance)?;
        // ...and call the entry point
        instance.call_handle_init(&mut store, "").await
    }

    async fn handle_module2_init_event(&self, component_id: &str) -> anyhow::Result<String> {
        // Load the guest...
        let (instance, mut store) = self.engine.prepare_instance(component_id).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };
        let instance = SpinOrchestratorModule2::new(&mut store, &instance)?;

        // ...and call the entry point
        instance.call_handle_init(&mut store, "").await
    }
}
