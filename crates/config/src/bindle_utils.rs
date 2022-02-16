
#![deny(missing_docs)]

use std::{fmt::Debug, path::Path};

use anyhow::{Result, Context};
use async_trait::async_trait;
use bindle::{Invoice, Label, Parcel};
use itertools::Itertools;
use reqwest::RequestBuilder;

static EMPTY: &Vec<bindle::Parcel> = &vec![];

// EXPLORATION: if an expanded spin.toml-a-like was in the bindle.
// Hypothetically, let's distinguish it by its media type (could also
// be done via a group or annotation or such like).

const SPIN_MANIFEST_MEDIA_TYPE: &str = "application/vnd.fermyon.spin+toml";

pub(crate) fn find_application_manifest(invoice: &Invoice) -> Result<String> {
    let manifest_parcels = invoice
        .parcel
        .as_ref()
        .unwrap_or(&EMPTY)
        .iter()
        .filter_map(|p|
            if p.label.media_type == SPIN_MANIFEST_MEDIA_TYPE {
                Some(&p.label)
            } else {
                None
            }
        )
        .collect_vec();

    match manifest_parcels.len() {
        0 => Err(anyhow::anyhow!("Invoice does not contain a Spin manifest")),
        1 => Ok(manifest_parcels[0].sha256.clone()),
        _ => Err(anyhow::anyhow!("Invoice contains multiple Spin manifests")),
    }
}

// This isn't currently transitive - I don't think we have a need for that
// but could add it if we did (WAGI has code for this but it's a huge faff)
pub(crate) fn parcels_in_group(invoice: &Invoice, group: &str) -> Vec<Label>{
    invoice
        .parcel
        .as_ref()
        .unwrap_or(&EMPTY)
        .iter()
        .filter_map(|p|
            if is_member_of(p, group) {
                Some(p.label.clone())
            } else {
                None
            }
        )
        .collect_vec()
}

fn is_member_of(parcel: &Parcel, group: &str) -> bool {
    if let Some(conditions) = &parcel.conditions {
        if let Some(member_of) = &conditions.member_of {
            return member_of.contains(&group.to_owned());
        }
    }
    false
}

// What changes do we need to make to the schema?
//
// application information -> shouldn't this come from the invoice rather than the manifest
//
// component ->
//   source -> should be a parcel sha
//   files -> could be an array of parcel shas, or the name of a group

/*

name        = "spin-hello-world"
version     = "1.0.0"
description = "A simple application that returns hello and goodbye."
authors     = [ "Radu Matei <radu@fermyon.com>" ]
trigger     = "http"

[[component]]
    source = "parcel_parcel_parcel"
    id     = "hello"
    files = group_group_group
[component.trigger]
    route = "/hello"


*/

#[derive(Clone)]
pub(crate) enum BindleTokenManager {
    NoToken(bindle::client::tokens::NoToken),
}

#[async_trait]
impl bindle::client::tokens::TokenManager for BindleTokenManager {
    async fn apply_auth_header(&self, builder: RequestBuilder) -> bindle::client::Result<RequestBuilder> {
        match self {
            Self::NoToken(t) => t.apply_auth_header(builder).await,
        }
    }
}

impl Debug for BindleTokenManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoToken(_) => f.debug_tuple("NoToken").finish(),
        }
    }
}

/// Encapsulate a Bindle source.
#[derive(Clone, Debug)]
pub struct BindleReader {
    inner: BindleReaderInner,
}

impl BindleReader {
    /// Gets the content of a parcel from the bindle source.
    pub async fn get_parcel(&self, parcel_id: &str) -> anyhow::Result<Vec<u8>> {
        match &self.inner {
            BindleReaderInner::Remote(client, bindle_id) =>
                client.get_parcel(bindle_id, parcel_id).await
                    .with_context(|| format!("Error fetching remote parcel {}@{}", bindle_id, parcel_id)),

            BindleReaderInner::Standalone(standalone) => {
                let path = standalone.parcel_dir.join(format!("{}.dat", parcel_id));
                tokio::fs::read(&path).await
                    .with_context(|| format!("Error reading standalone parcel {} from {}", parcel_id, path.display()))
            }
        }
    }

    /// Get the invoice from the bindle source
    pub async fn get_invoice(&self) -> anyhow::Result<bindle::Invoice> {
        match &self.inner {
            BindleReaderInner::Remote(client, bindle_id) =>
                client.get_invoice(bindle_id).await
                    .with_context(|| format!("Error fetching remote invoice {}", bindle_id)),

            BindleReaderInner::Standalone(standalone) => {
                let invoice_bytes = tokio::fs::read(&standalone.invoice_file).await
                    .with_context(|| format!("Error reading bindle invoice rom '{}'", standalone.invoice_file.display()))?;
                toml::from_slice(&invoice_bytes)
                    .with_context(|| format!("Error parsing file '{}' as invoice", standalone.invoice_file.display()))
            }
        }
    }

    pub(crate) fn remote(client: &bindle::client::Client<BindleTokenManager>, bindle_id: &bindle::Id) -> Self {
        Self {
            inner: BindleReaderInner::Remote(client.clone(), bindle_id.clone())
        }
    }

    #[allow(dead_code)]  // for now
    pub(crate) async fn standalone(base_path: impl AsRef<Path>, bindle_id: &bindle::Id) -> Result<Self> {
        let standalone = bindle::standalone::StandaloneRead::new(&base_path, bindle_id).await?;
        Ok(Self {
            inner: BindleReaderInner::Standalone(std::sync::Arc::new(standalone))
        })
    }
}

#[derive(Clone)]
enum BindleReaderInner {
    Standalone(std::sync::Arc<bindle::standalone::StandaloneRead>),
    Remote(bindle::client::Client<BindleTokenManager>, bindle::Id),
}

impl Debug for BindleReaderInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standalone(_) => f.debug_tuple("Standalone").finish(),
            Self::Remote(_, _) => f.debug_tuple("Remote").finish(),
        }
    }
}
