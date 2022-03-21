use anyhow::Result;
use spin_config::{
    ApplicationInformation, ApplicationOrigin, Configuration, CoreComponent, ModuleSource,
    TriggerConfig, WasmConfig,
};
use spin_timer_echo::TimerTrigger;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let trigger = TimerTrigger::new(Duration::from_secs(1), app()).await?;
    trigger.run().await
}

fn app() -> Configuration<CoreComponent> {
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

    Configuration::<CoreComponent> { info, components }
}
