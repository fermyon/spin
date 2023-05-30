//! Functionality to get a prepared Spin application from Bindle.

#![deny(missing_docs)]

/// Module to prepare the assets for the components of an application.
mod assets;
/// Configuration representation for a Spin application in Bindle.
pub mod config;
mod connection;
/// Functions relating to Bindle deprecation.
pub mod deprecation;
/// Bindle helper functions.
mod utils;

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use bindle::Invoice;
use futures::future;
use outbound_http::allowed_http_hosts::validate_allowed_http_hosts;
use spin_manifest::{
    Application, ApplicationInformation, ApplicationOrigin, CoreComponent, ModuleSource,
    SpinVersion, WasmConfig,
};

use crate::bindle::{
    config::{RawAppManifest, RawComponentManifest},
    utils::{find_manifest, parcels_in_group},
};
pub use connection::BindleConnectionInfo;
pub(crate) use utils::BindleReader;
pub use utils::SPIN_MANIFEST_MEDIA_TYPE;

/// Given a Bindle server URL and reference, pull it, expand its assets locally, and get a
/// prepared application configuration consumable by a Spin execution context.
/// If a directory is provided, use it as the base directory to expand the assets,
/// otherwise create a new temporary directory.
pub async fn from_bindle(id: &str, url: &str, base_dst: impl AsRef<Path>) -> Result<Application> {
    // TODO
    // Handle Bindle authentication.
    let connection_info = BindleConnectionInfo::new(url, false, None, None);
    let client = connection_info.client()?;
    let reader = BindleReader::remote(&client, &id.parse()?);

    prepare(id, url, &reader, base_dst).await
}

/// Converts a Bindle invoice into Spin configuration.
async fn prepare(
    id: &str,
    url: &str,
    reader: &BindleReader,
    base_dst: impl AsRef<Path>,
) -> Result<Application> {
    // First, get the invoice from the Bindle server.
    let invoice = reader
        .get_invoice()
        .await
        .with_context(|| anyhow!("Failed to load invoice '{}' from '{}'", id, url))?;

    // Then, reconstruct the application manifest from the parcels.
    let raw: RawAppManifest =
        toml::from_slice(&reader.get_parcel(&find_manifest(&invoice)?).await?)?;
    tracing::trace!("Recreated manifest from bindle: {:?}", raw);

    validate_raw_app_manifest(&raw)?;

    let info = info(&raw, &invoice, url);
    tracing::trace!("Application information from bindle: {:?}", info);
    let component_triggers = raw
        .components
        .iter()
        .map(|c| (c.id.clone(), c.trigger.clone()))
        .collect();
    let components = future::join_all(
        raw.components
            .into_iter()
            .map(|c| async { core(c, &invoice, reader, &base_dst).await })
            .collect::<Vec<_>>(),
    )
    .await
    .into_iter()
    .map(|x| x.expect("Cannot prepare component"))
    .collect::<Vec<_>>();

    let variables = raw
        .variables
        .into_iter()
        .map(|(key, var)| Ok((key, var.try_into()?)))
        .collect::<Result<_>>()?;

    Ok(Application {
        info,
        variables,
        components,
        component_triggers,
    })
}

fn validate_raw_app_manifest(raw: &RawAppManifest) -> Result<()> {
    raw.components
        .iter()
        .try_for_each(|c| validate_allowed_http_hosts(&c.wasm.allowed_http_hosts))
}

/// Given a raw component manifest, prepare its assets and return a fully formed core component.
async fn core(
    raw: RawComponentManifest,
    invoice: &Invoice,
    reader: &BindleReader,
    base_dst: impl AsRef<Path>,
) -> Result<CoreComponent> {
    let bytes = reader
        .get_parcel(&raw.source)
        .await
        .with_context(|| anyhow!("Cannot get module source from bindle"))?;

    let source = ModuleSource::Buffer(bytes, format!("parcel {}", raw.source));
    let id = raw.id;
    let description = raw.description;
    let mounts = match raw.wasm.files {
        Some(group) => {
            let parcels = parcels_in_group(invoice, &group);
            vec![
                assets::prepare_component(reader, &invoice.bindle.id, &parcels, base_dst, &id)
                    .await?,
            ]
        }
        None => vec![],
    };
    let environment = raw.wasm.environment.unwrap_or_default();
    let allowed_http_hosts = raw.wasm.allowed_http_hosts.unwrap_or_default();
    let key_value_stores = raw.wasm.key_value_stores.unwrap_or_default();
    let sqlite_databases = raw.wasm.sqlite_databases.unwrap_or_default();
    let wasm = WasmConfig {
        environment,
        mounts,
        allowed_http_hosts,
        key_value_stores,
        sqlite_databases,
    };
    let config = raw.config.unwrap_or_default();
    Ok(CoreComponent {
        source,
        id,
        description,
        wasm,
        config,
    })
}

/// Converts the raw application manifest from the bindle invoice into the
/// standard application configuration.
fn info(raw: &RawAppManifest, invoice: &Invoice, url: &str) -> ApplicationInformation {
    ApplicationInformation {
        // TODO
        // Handle API version
        spin_version: SpinVersion::V1,
        name: invoice.bindle.id.name().to_string(),
        version: invoice.bindle.id.version_string(),
        description: invoice.bindle.description.clone(),
        authors: invoice.bindle.authors.clone().unwrap_or_default(),
        trigger: raw.trigger.clone(),
        origin: ApplicationOrigin::Bindle {
            id: invoice.bindle.id.to_string(),
            server: url.to_string(),
        },
    }
}
