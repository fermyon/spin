//! This crates contains common code for use in tests. Many methods will panic
//! in the slightest breeze, so DO NOT USE IN NON-TEST CODE.

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Once,
};

use http::{uri::Scheme, Request, Response};
use hyper::Body;
use serde::de::DeserializeOwned;
use spin_app::{
    async_trait,
    locked::{LockedApp, LockedComponentSource},
    AppComponent, Loader,
};
use spin_core::{Module, StoreBuilder};
use spin_http_engine::{HttpExecutorType, HttpTrigger, HttpTriggerConfig, WagiTriggerConfig};
use spin_trigger::{TriggerExecutor, TriggerExecutorBuilder};

static LOGGER: Once = Once::new();

/// We can only initialize the tracing subscriber once per crate.
pub(crate) fn init() {
    LOGGER.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .init();
    });
}

macro_rules! from_json {
    ($($json:tt)+) => {
        serde_json::from_value(serde_json::json!($($json)+)).expect("valid json")
    };
}

#[derive(Default)]
pub struct TestConfig {
    module_path: Option<PathBuf>,
    http_trigger_config: HttpTriggerConfig,
    redis_channel: String,
}

impl TestConfig {
    pub fn module_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.module_path = Some(path.into());
        self
    }

    pub fn test_program(&mut self, name: impl AsRef<Path>) -> &mut Self {
        self.module_path(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/test-programs")
                .join(name),
        )
    }

    pub fn http_spin_trigger(&mut self, route: impl Into<String>) -> &mut Self {
        self.http_trigger_config = HttpTriggerConfig {
            component: "test-component".to_string(),
            route: route.into(),
            executor: None,
        };
        self
    }

    pub fn http_wagi_trigger(
        &mut self,
        route: impl Into<String>,
        wagi_config: WagiTriggerConfig,
    ) -> &mut Self {
        self.http_trigger_config = HttpTriggerConfig {
            component: "test-component".to_string(),
            route: route.into(),
            executor: Some(HttpExecutorType::Wagi(wagi_config)),
        };
        self
    }

    pub fn redis_trigger(&mut self, channel: impl Into<String>) -> &mut Self {
        self.redis_channel = channel.into();
        self
    }

    pub fn build_locked_app(&self) -> LockedApp {
        let components = from_json!([{
            "id": "test-component",
            "source": {
                "content_type": "application/wasm",
                "digest": "test-source",
            },
        }]);
        let triggers = from_json!([
            {
                "id": "test-http-trigger",
                "trigger_type": "http",
                "trigger_config": self.http_trigger_config,
            },
                {
                "id": "test-redis-trigger",
                "trigger_type": "redis",
                "trigger_config": {
                    "component": "test-component",
                    "channel": self.redis_channel,
                },
            },
        ]);
        let metadata = from_json!({"name": "test-app", "redis_address": "test-redis-host"});
        let variables = Default::default();
        LockedApp {
            spin_lock_version: spin_app::locked::FixedVersion,
            components,
            triggers,
            metadata,
            variables,
        }
    }

    pub fn build_loader(&self) -> impl Loader {
        init();
        TestLoader {
            app: self.build_locked_app(),
            module_path: self.module_path.clone().expect("module path to be set"),
        }
    }

    pub async fn build_trigger<Executor: TriggerExecutor>(&self) -> Executor
    where
        Executor::TriggerConfig: DeserializeOwned,
    {
        TriggerExecutorBuilder::new(self.build_loader())
            .build(TEST_APP_URI.to_string())
            .await
            .unwrap()
    }

    pub async fn build_http_trigger(&self) -> HttpTrigger {
        self.build_trigger().await
    }

    pub async fn handle_http_request(&self, req: Request<Body>) -> anyhow::Result<Response<Body>> {
        self.build_http_trigger()
            .await
            .handle(req, Scheme::HTTP, test_socket_addr())
            .await
    }
}

const TEST_APP_URI: &str = "spin-test:";

struct TestLoader {
    app: LockedApp,
    module_path: PathBuf,
}

#[async_trait]
impl Loader for TestLoader {
    async fn load_app(&self, uri: &str) -> anyhow::Result<LockedApp> {
        assert_eq!(uri, TEST_APP_URI);
        Ok(self.app.clone())
    }

    async fn load_module(
        &self,
        engine: &spin_core::wasmtime::Engine,
        source: &LockedComponentSource,
    ) -> anyhow::Result<spin_core::Module> {
        assert_eq!(source.content.digest.as_deref(), Some("test-source"),);
        Module::from_file(engine, &self.module_path)
    }

    async fn mount_files(
        &self,
        _store_builder: &mut StoreBuilder,
        component: &AppComponent,
    ) -> anyhow::Result<()> {
        assert_eq!(component.files().len(), 0, "files testing not implemented");
        Ok(())
    }
}

pub fn test_socket_addr() -> SocketAddr {
    "127.0.0.1:55555".parse().unwrap()
}

pub fn assert_http_response_success(resp: &Response<Body>) {
    if !resp.status().is_success() {
        panic!("non-success response {}: {:?}", resp.status(), resp.body());
    }
}
