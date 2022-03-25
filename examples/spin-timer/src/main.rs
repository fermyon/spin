// The wit_bindgen_wasmtime::import below is triggering this lint.
#![allow(clippy::needless_question_mark)]

use anyhow::Result;
use spin_config::{
    Application, ApplicationInformation, ApplicationOrigin, CoreComponent, ModuleSource,
    TriggerConfig, WasmConfig,
};
use spin_engine::{Builder, ExecutionContextConfiguration};
use std::{sync::Arc, time::Duration};
use tokio::task::spawn_blocking;

wit_bindgen_wasmtime::import!("spin-timer.wit");

type ExecutionContext = spin_engine::ExecutionContext<spin_timer::SpinTimerData>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let trigger = TimerTrigger::new(Duration::from_secs(1), app()).await?;
    trigger.run().await
}

/// A custom timer trigger that executes the
/// first component of an application on every interval.
#[derive(Clone)]
pub struct TimerTrigger {
    /// The interval at which the component is executed.
    pub interval: Duration,
    /// The application configuration.
    app: Application<CoreComponent>,
    /// The Spin execution context.
    engine: Arc<ExecutionContext>,
}

impl TimerTrigger {
    /// Creates a new trigger.
    pub async fn new(interval: Duration, app: Application<CoreComponent>) -> Result<Self> {
        let config = ExecutionContextConfiguration::new(app.clone(), None);
        let engine = Arc::new(Builder::build_default(config).await?);
        log::debug!("Created new Timer trigger.");

        Ok(Self {
            interval,
            app,
            engine,
        })
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
        let (mut store, instance) =
            self.engine
                .prepare_component(&self.app.components[0].id, None, None, None, None)?;

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

pub fn app() -> Application<CoreComponent> {
    let info = ApplicationInformation {
        spin_version: spin_config::SpinVersion::V1,
        name: "test-app".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        authors: vec![],
        trigger: spin_config::ApplicationTrigger::Http(spin_config::HttpTriggerConfiguration {
            base: "/".to_owned(),
        }),
        namespace: None,
        origin: ApplicationOrigin::File("".into()),
    };

    let component = CoreComponent {
        source: ModuleSource::FileReference("target/test-programs/echo.wasm".into()),
        id: "test".to_string(),
        trigger: TriggerConfig::default(),
        wasm: WasmConfig::default(),
    };
    let components = vec![component];

    Application::<CoreComponent> { info, components }
}
