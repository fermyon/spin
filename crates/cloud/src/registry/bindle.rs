#![deny(missing_docs)]

//! Functions for publishing Spin applications& to Bindle.

mod bindle_pusher;
mod bindle_writer;
mod expander;

pub use bindle_writer::write;
pub use expander::expand_manifest;

const BINDLE_REGISTRY_URL_PATH: &str = "api/registry";

use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use bindle::client::{
    tokens::{LongLivedToken, TokenManager},
    Client, ClientBuilder,
};
use semver::BuildMetadata;
use tracing::log;

use crate::client::ConnectionConfig;

use self::bindle_pusher::push_all;

/// Publish the application to the Cloud's Bindle server.
pub async fn publish(
    path: impl AsRef<Path>,
    buildinfo: Option<BuildMetadata>,
    connection: ConnectionConfig,
) -> Result<(String, String)> {
    let source_dir = path
        .as_ref()
        .parent()
        .context("Failed to get source directory")?;

    let info = BindleConnectionInfo::new(
        format!("{}/{}", connection.url, BINDLE_REGISTRY_URL_PATH),
        connection.insecure,
        connection.token.token.context("Failed to get token")?,
    );

    log::trace!(
        "Deploying application from {:?} to {}",
        source_dir,
        info.base_url
    );

    let tmp = tempfile::tempdir().context("Cannot create temporary directory")?;
    let dest_dir = tmp.path();

    let (mut invoice, sources) = spin_publish::expand_manifest(&path, buildinfo, &dest_dir)
        .await
        .with_context(|| format!("Failed to expand '{:?}' to a bindle", &dest_dir))?;

    // This is intended to make sure all applications are namespaced using the Fermyon user account.
    // TODO: This should check whether the invoice is already namespaced.
    invoice.bindle.id = format!(
        "{}/{}",
        invoice.bindle.id.name(),
        invoice.bindle.id.version()
    )
    .parse()?;

    let bindle_id = &invoice.bindle.id;

    spin_publish::write(&source_dir, &dest_dir, &invoice, &sources)
        .await
        .with_context(|| write_failed_msg(bindle_id, dest_dir))?;

    push_all(&dest_dir, bindle_id, info.clone()).await?;

    log::trace!("Published to {:?}", invoice.bindle.id);

    Ok((bindle_id.name().into(), bindle_id.version_string()))
}

/// BindleConnectionInfo holds the details of a connection to a
/// Bindle server, including url, insecure configuration and an
/// auth token manager
#[derive(Clone)]
pub(crate) struct BindleConnectionInfo {
    pub(crate) base_url: String,
    pub(crate) allow_insecure: bool,
    pub(crate) token_manager: AnyAuth,
}

impl BindleConnectionInfo {
    /// Generates a new BindleConnectionInfo instance using the provided
    /// base_url, allow_insecure setting and token.
    pub(crate) fn new<I: Into<String>>(base_url: I, allow_insecure: bool, token: I) -> Self {
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
    pub(crate) fn client(&self) -> bindle::client::Result<Client<AnyAuth>> {
        let builder = ClientBuilder::default()
            .http2_prior_knowledge(false)
            .danger_accept_invalid_certs(self.allow_insecure);
        builder.build(&self.base_url, self.token_manager.clone())
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

pub(crate) fn write_failed_msg(bindle_id: &bindle::Id, dest_dir: &Path) -> String {
    format!(
        "Failed to write bindle '{}' to {}",
        bindle_id,
        dest_dir.display()
    )
}
