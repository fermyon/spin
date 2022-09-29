// The wit_bindgen_wasmtime::import below is triggering this lint.
#![allow(clippy::needless_question_mark)]

use std::time::Duration;

use anyhow::Result;
use spin_core::{Engine, InstancePre, Module};

wit_bindgen_wasmtime::import!({paths: ["spin-timer.wit"], async: *});

type RuntimeData = spin_timer::SpinTimerData;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let engine = Engine::builder(&Default::default())?.build();

    let module = Module::from_file(
        engine.as_ref(),
        "example/target/wasm32-wasi/release/rust_echo_test.wasm",
    )?;

    let instance_pre = engine.instantiate_pre(&module)?;

    let trigger = TimerTrigger {
        engine,
        instance_pre,
        interval: Duration::from_secs(1),
    };

    trigger.run().await
}

/// A custom timer trigger that executes a component on
/// every interval.
pub struct TimerTrigger {
    /// The Spin core engine.
    pub engine: Engine<RuntimeData>,
    /// The pre-initialized component instance to execute.
    pub instance_pre: InstancePre<RuntimeData>,
    /// The interval at which the component is executed.
    pub interval: Duration,
}

impl TimerTrigger {
    /// Runs the trigger at every interval.
    async fn run(&self) -> Result<()> {
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
        let mut store = self.engine.store_builder().build()?;
        let instance = self.instance_pre.instantiate_async(&mut store).await?;
        let timer_instance =
            spin_timer::SpinTimer::new(&mut store, &instance, |data| data.as_mut())?;
        let res = timer_instance
            .handle_timer_request(&mut store, &msg)
            .await?;
        tracing::info!("{}\n", res);

        Ok(())
    }
}
