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
    path: "spin-orchestrator-modules.wit",
    world: "spin-orchestrator-modules.spin-orchestrator-module1",
    async: true
});

// Module 2 WIT bindings
wasmtime::component::bindgen!({
    path: "spin-orchestrator-modules.wit",
    world: "spin-orchestrator-modules.spin-orchestrator-module2",
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
                let moudle1_output = self.handle_module1_init_event(component, "init_message").await?;
                self.handle_module2_init_event(component, &moudle1_output).await?;
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
                // Orchestrate components here e.g. module1 sends it's output to module2                
                // Validate if only the required modules are present in the config.
                if self.component_timings.len() == 2 &&
                    self.component_timings.contains_key("module1") 
                    && self.component_timings.contains_key("module2")
                {
                    let module1_config = &self.component_timings["module1"];
                    let config: Module1TriggerConfig = serde_json::from_str(module1_config).unwrap();
                    println!("module1: Config: {:?}", config);
                    let module2_config = &self.component_timings["module2"];
                    let config: Module2TriggerConfig = serde_json::from_str(module2_config).unwrap();
                    println!("module2: Config: {:?}", config);

                    scope.spawn(async {
                        let module1_output = self.handle_module1_init_event("module1", "Init Message").await.unwrap();
                        println!("module1 output: {:?}", module1_output);
                        let module2_output = self.handle_module2_init_event("module2", &module1_output).await.unwrap();
                        println!("module2 output: {:?}", module2_output);
                    });
                }
                else {
                    panic!("Unknown modules confiugured in application: {:?}", &self.component_timings.keys());
                }              
            });
        }
        Ok(())
    }
}

impl OrchestratorTrigger {
    async fn handle_module1_init_event(&self, component_id: &str, init_message: &str) -> anyhow::Result<String> {
        // Load the guest...
        let (instance, mut store) = self.engine.prepare_instance(component_id).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };

        let instance = SpinOrchestratorModule1::new(&mut store, &instance)?;

        // ...and call the entry point
        instance.call_handle_init(&mut store, init_message).await
    }

    async fn handle_module2_init_event(&self, component_id: &str, module1_output: &str) -> anyhow::Result<String> {
        // Load the guest...
        let (instance, mut store) = self.engine.prepare_instance(component_id).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };
        let instance = SpinOrchestratorModule2::new(&mut store, &instance)?;

        // ...and call the entry point
        instance.call_handle_init(&mut store, module1_output).await
    }
}
