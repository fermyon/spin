//! This crates contains common code for use in tests. Many methods will panic
//! in the slightest breeze, so DO NOT USE IN NON-TEST CODE.

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use http::{Request, Response};
use hyper::Body;
use spin_engine::{io::FollowComponents, Builder};
use spin_http_engine::HttpTrigger;
use spin_manifest::{
    Application, ApplicationInformation, ApplicationOrigin, ApplicationTrigger, CoreComponent,
    HttpConfig, ModuleSource, RedisConfig, RedisTriggerConfiguration, SpinVersion, TriggerConfig,
};

#[derive(Default)]
pub struct TestConfig {
    module_path: Option<PathBuf>,
    application_trigger: Option<ApplicationTrigger>,
    trigger_config: Option<TriggerConfig>,
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

    pub fn http_trigger(&mut self, config: HttpConfig) -> &mut Self {
        self.application_trigger = Some(ApplicationTrigger::Http(Default::default()));
        self.trigger_config = Some(TriggerConfig::Http(config));
        self
    }

    pub fn redis_trigger(&mut self, config: RedisConfig) -> &mut Self {
        self.application_trigger = Some(ApplicationTrigger::Redis(RedisTriggerConfiguration {
            address: "redis://localhost:6379".to_owned(),
        }));
        self.trigger_config = Some(TriggerConfig::Redis(config));
        self
    }

    pub fn build_application_information(&self) -> ApplicationInformation {
        ApplicationInformation {
            spin_version: SpinVersion::V1,
            name: "test-app".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            authors: vec![],
            trigger: self
                .application_trigger
                .clone()
                .expect("http_trigger or redis_trigger required"),
            namespace: None,
            origin: ApplicationOrigin::File(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fake_spin.toml"),
            ),
        }
    }

    pub fn build_component(&self) -> CoreComponent {
        let module_path = self
            .module_path
            .clone()
            .expect("module_path or test_program required");
        CoreComponent {
            source: ModuleSource::FileReference(module_path),
            id: "test-component".to_string(),
            description: None,
            wasm: Default::default(),
        }
    }

    pub fn build_application(&self) -> Application {
        Application {
            info: self.build_application_information(),
            components: vec![self.build_component()],
            component_triggers: [(
                "test-component".to_string(),
                self.trigger_config
                    .clone()
                    .expect("http_trigger or redis_trigger required"),
            )]
            .into_iter()
            .collect(),
            config_resolver: None,
        }
    }

    pub async fn prepare_builder<T: Default + 'static>(&self, app: Application) -> Builder<T> {
        let mut builder = Builder::new(app.into()).expect("Builder::new failed");
        builder.link_defaults().expect("link_defaults failed");
        builder
    }

    pub async fn build_http_trigger(&self) -> HttpTrigger {
        let app = self.build_application();
        spin_trigger::build_trigger_from_app(app, None, None, FollowComponents::None, None)
            .await
            .unwrap()
    }

    pub async fn handle_http_request(&self, req: Request<Body>) -> anyhow::Result<Response<Body>> {
        self.build_http_trigger()
            .await
            .handle(req, test_socket_addr())
            .await
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
