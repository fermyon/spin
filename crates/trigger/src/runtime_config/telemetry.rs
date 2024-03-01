use tracing::subscriber::{DefaultGuard, NoSubscriber, Subscriber};
use url::Url;

use crate::RuntimeConfig;

#[derive(Debug, serde::Deserialize)]
pub struct OtlpOpts {
    endpoint: Url,
}

pub fn set_subscriber(runtime_config: &RuntimeConfig) -> DefaultGuard {
    tracing::subscriber::set_default(NoSubscriber::new())
}
