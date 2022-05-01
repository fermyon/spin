// The wit_bindgen_wasmtime::import below is triggering this lint.
#![allow(clippy::needless_question_mark)]

use anyhow::Result;
use async_trait::async_trait;
use spin_engine::{Builder, ExecutionContextConfiguration};
use spin_manifest::{Application, ComponentMap, CoreComponent, ModuleSource, WasmConfig};
use spin_timer::SpinTimerData;
use spin_trigger::Trigger;
use std::{sync::Arc, time::Duration};
use tokio::task::spawn_blocking;

wit_bindgen_wasmtime::import!("spin-timer.wit");

type ExecutionContext = spin_engine::ExecutionContext<spin_timer::SpinTimerData>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let component = component();
    let builder = Builder::build_default(ExecutionContextConfiguration {
        components: vec![component],
        label: "timer-app".to_string(),
        ..Default::default()
    })
    .await?;
    let trigger = TimerTrigger::new(builder, (), Default::default(), ())?;
    trigger
        .run(TimerRuntimeConfig {
            interval: Duration::from_secs(1),
        })
        .await
}

/// A custom timer trigger that executes the
/// first component of an application on every interval.
#[derive(Clone)]
pub struct TimerTrigger {
    /// The Spin execution context.
    engine: Arc<ExecutionContext>,
}

#[derive(Clone)]
pub struct TimerRuntimeConfig {
    /// The interval at which the component is executed.   
    pub interval: Duration,
}

#[async_trait]
impl Trigger for TimerTrigger {
    type ContextData = SpinTimerData;
    type Config = ();
    type ComponentConfig = ();
    type RuntimeConfig = TimerRuntimeConfig;
    type TriggerExtra = ();

    /// Creates a new trigger.
    fn new(
        execution_context: ExecutionContext,
        _: Self::Config,
        _: ComponentMap<Self::ComponentConfig>,
        _: Self::TriggerExtra,
    ) -> Result<Self> {
        Ok(Self {
            engine: Arc::new(execution_context),
        })
    }

    fn build_trigger_extra(_app: Application<CoreComponent>) -> Result<Self::TriggerExtra> {
        Ok(())
    }
    /// Runs the trigger at every interval.
    async fn run(&self, run_config: Self::RuntimeConfig) -> Result<()> {
        let mut interval = tokio::time::interval(run_config.interval);
        loop {
            interval.tick().await;
            self.handle(
                chrono::Local::now()
                    .format("%Y-%m-%d][%H:%M:%S")
                    .to_string(),
            )
            .await?;
        }
    }
}

impl TimerTrigger {
    /// Execute the first component in the application configuration.
    async fn handle(&self, msg: String) -> Result<()> {
        let (mut store, instance) = self.engine.prepare_component(
            &self.engine.config.components[0].id,
            None,
            None,
            None,
            None,
        )?;

        let res = spawn_blocking(move || -> Result<String> {
            let t = spin_timer::SpinTimer::new(&mut store, &instance, |host| {
                host.data.as_mut().unwrap()
            })?;
            Ok(t.handle_timer_request(&mut store, &msg)?)
        })
        .await??;
        log::info!("{}\n", res);

        Ok(())
    }
}

pub fn component() -> CoreComponent {
    CoreComponent {
        source: ModuleSource::FileReference("target/test-programs/echo.wasm".into()),
        id: "test".to_string(),
        wasm: WasmConfig::default(),
    }
}
