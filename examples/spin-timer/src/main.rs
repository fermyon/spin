// The wit_bindgen_wasmtime::import below is triggering this lint.
#![allow(clippy::needless_question_mark)]

use anyhow::Result;
use spin_engine::{Builder, ExecutionContextConfiguration};
use spin_manifest::{CoreComponent, ModuleSource, WasmConfig};
use std::{sync::Arc, time::Duration};
use tokio::task::spawn_blocking;

wit_bindgen_wasmtime::import!("spin-timer.wit");

type ExecutionContext = spin_engine::ExecutionContext<spin_timer::SpinTimerData>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let trigger = TimerTrigger::new(Duration::from_secs(1), component()).await?;
    trigger.run().await
}

/// A custom timer trigger that executes the
/// first component of an application on every interval.
#[derive(Clone)]
pub struct TimerTrigger {
    /// The interval at which the component is executed.
    pub interval: Duration,
    /// The Spin execution context.
    engine: Arc<ExecutionContext>,
}

impl TimerTrigger {
    /// Creates a new trigger.
    pub async fn new(interval: Duration, component: CoreComponent) -> Result<Self> {
        let config = ExecutionContextConfiguration {
            components: vec![component],
            label: "timer-app".to_string(),
            ..Default::default()
        };
        let engine = Arc::new(Builder::build_default(config).await?);
        log::debug!("Created new Timer trigger.");

        Ok(Self { interval, engine })
    }

    /// Runs the trigger at every interval.
    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(self.interval);
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
        description: None,
        wasm: WasmConfig::default(),
    }
}
