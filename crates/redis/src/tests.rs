use super::*;
use anyhow::Result;
use spin_config::{
    ApplicationInformation, Configuration, CoreComponent, ModuleSource, RedisConfig, RedisExecutor,
    SpinVersion, TriggerConfig,
};
use std::{collections::HashMap, sync::Once};

static LOGGER: Once = Once::new();

const RUST_ENTRYPOINT_PATH: &str = "../../target/test-programs/redis-rust.wasm";

/// We can only initialize the tracing subscriber once per crate.
pub(crate) fn init() {
    LOGGER.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    });
}

fn fake_file_origin() -> spin_config::ApplicationOrigin {
    let dir = env!("CARGO_MANIFEST_DIR");
    let fake_path = std::path::PathBuf::from(dir).join("fake_spin.toml");
    spin_config::ApplicationOrigin::File(fake_path)
}

#[tokio::test]
#[allow(unused)]
async fn test_pubsub() -> Result<()> {
    init();

    let info = ApplicationInformation {
        spin_version: SpinVersion::V1,
        name: "test-redis".to_string(),
        version: "0.1.0".to_string(),
        description: None,
        authors: vec![],
        trigger: spin_config::ApplicationTrigger::Redis(spin_config::RedisTriggerConfiguration {
            address: "redis://localhost:6379".to_owned(),
        }),
        namespace: None,
        origin: fake_file_origin(),
    };

    let components = vec![CoreComponent {
        source: ModuleSource::FileReference(RUST_ENTRYPOINT_PATH.into()),
        id: "test".to_string(),
        trigger: TriggerConfig::Redis(RedisConfig {
            channel: "messages".to_string(),
            executor: Some(RedisExecutor::Spin),
        }),
        wasm: spin_config::WasmConfig {
            environment: HashMap::new(),
            mounts: vec![],
            allowed_http_hosts: vec![],
        },
    }];

    let app = Configuration::<CoreComponent> { info, components };
    let trigger = RedisTrigger::new(app, None, None).await?;

    // TODO
    // use redis::{FromRedisValue, Msg, Value};
    // let val = FromRedisValue::from_redis_value(&Value::Data("hello".into()))?;
    // let msg = Msg::from_value(&val).unwrap();
    // trigger.handle(msg).await?;

    Ok(())
}
