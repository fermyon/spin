#![deny(missing_docs)]

//! Functions for publishing Spin applications to Bindle.

mod bindle_pusher;
mod bindle_writer;
mod expander;

pub use bindle_pusher::push_all;
pub use bindle_writer::write;
pub use expander::expand_manifest;

use bindle::client::{
    tokens::{HttpBasic, LongLivedToken, NoToken, TokenManager},
    Client, ClientBuilder,
};
use std::sync::Arc;

/// BindleConnectionInfo holds the details of a connection to a
/// Bindle server, including url, insecure configuration and an
/// auth token manager
#[derive(Clone)]
pub struct BindleConnectionInfo {
    base_url: String,
    allow_insecure: bool,
    token_manager: AnyAuth,
}

impl BindleConnectionInfo {
    /// Generates a new BindleConnectionInfo instance using the provided
    /// base_url, allow_insecure setting and optional username and password
    /// for basic http auth
    pub fn new<I: Into<String>>(
        base_url: I,
        allow_insecure: bool,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        let token_manager: Box<dyn TokenManager + Send + Sync> = match (username, password) {
            (Some(u), Some(p)) => Box::new(HttpBasic::new(&u, &p)),
            _ => Box::new(NoToken::default()),
        };

        Self {
            base_url: base_url.into(),
            allow_insecure,
            token_manager: AnyAuth {
                token_manager: Arc::new(token_manager),
            },
        }
    }

    /// Generates a new BindleConnectionInfo instance using the provided
    /// base_url, allow_insecure setting and token.
    pub fn from_token<I: Into<String>>(base_url: I, allow_insecure: bool, token: I) -> Self {
        let token_manager: Box<dyn TokenManager + Send + Sync> =
            Box::new(LongLivedToken::new(&token.into()));

        Self {
            base_url: base_url.into(),
            allow_insecure,
            token_manager: AnyAuth {
                token_manager: Arc::new(token_manager),
            },
        }
    }

    /// Returns a client based on this instance's configuration
    pub fn client(&self) -> bindle::client::Result<Client<AnyAuth>> {
        let builder = ClientBuilder::default()
            .http2_prior_knowledge(false)
            .danger_accept_invalid_certs(self.allow_insecure);
        builder.build(&self.base_url, self.token_manager.clone())
    }

    /// Returns the base url for this client.
    pub fn base_url(&self) -> &str {
        self.base_url.as_ref()
    }
}

/// AnyAuth wraps an authentication token manager which applies
/// the appropriate auth header per its configuration
#[derive(Clone)]
pub struct AnyAuth {
    token_manager: Arc<Box<dyn TokenManager + Send + Sync>>,
}

#[async_trait::async_trait]
impl TokenManager for AnyAuth {
    async fn apply_auth_header(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> bindle::client::Result<reqwest::RequestBuilder> {
        self.token_manager.apply_auth_header(builder).await
    }
}
