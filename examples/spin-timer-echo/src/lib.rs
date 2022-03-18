use anyhow::Result;
use spin_config::{Configuration, CoreComponent};
use spin_engine::{Builder, ExecutionContextConfiguration};
use std::{sync::Arc, time::Duration};
use tokio::task::spawn_blocking;

wit_bindgen_wasmtime::import!("echo.wit");

type ExecutionContext = spin_engine::ExecutionContext<echo::EchoData>;

/// A custom timer trigger that executes the
/// first component of an application on every interval.
#[derive(Clone)]
pub struct TimerTrigger {
    /// The interval at which the component is executed.
    pub interval: Duration,
    /// The application configuration.
    app: Configuration<CoreComponent>,
    /// The Spin execution context.
    engine: Arc<ExecutionContext>,
}

impl TimerTrigger {
    /// Creates a new trigger.
    pub async fn new(interval: Duration, app: Configuration<CoreComponent>) -> Result<Self> {
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
            let e = echo::Echo::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;
            Ok(e.echo(&mut store, &msg)?)
        })
        .await??;
        log::info!("{}\n", res);

        Ok(())
    }
}
