#[cfg(feature = "runtime")]
mod host_component;
#[cfg(feature = "runtime")]
mod host_impl;

#[cfg(feature = "runtime")]
pub use host_component::OutboundHttpComponent;
#[cfg(feature = "runtime")]
pub use host_impl::{HttpRequestHandler, HttpSelfDispatcher};

use spin_locked_app::MetadataKey;

pub const ALLOWED_HTTP_HOSTS_KEY: MetadataKey<Vec<String>> = MetadataKey::new("allowed_http_hosts");
