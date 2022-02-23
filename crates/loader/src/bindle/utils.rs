#![deny(missing_docs)]

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use bindle::{
    client::{
        self,
        tokens::{NoToken, TokenManager},
        Client,
    },
    standalone::StandaloneRead,
    Id, Invoice, Label, Parcel,
};
use itertools::Itertools;
use reqwest::RequestBuilder;
use std::{fmt::Debug, path::Path, sync::Arc};
use tokio::fs;

static EMPTY: &Vec<bindle::Parcel> = &vec![];

// Alternative to storing `spin.toml` as a parcel, this could be
// distinguished it through a group, or an annotation.

/// The media type of a `spin.toml` parcel as part of a bindle.
pub(crate) const SPIN_MANIFEST_MEDIA_TYPE: &str = "application/vnd.fermyon.spin+toml";

pub(crate) fn find_manifest(inv: &Invoice) -> Result<String> {
    let parcels = inv
        .parcel
        .as_ref()
        .unwrap_or(EMPTY)
        .iter()
        .filter_map(|p| {
            if p.label.media_type == SPIN_MANIFEST_MEDIA_TYPE {
                Some(&p.label)
            } else {
                None
            }
        })
        .collect_vec();

    match parcels.len() {
        0 => bail!("Invoice does not contain a Spin manifest"),
        1 => Ok(parcels[0].sha256.clone()),
        _ => bail!("Invoice contains multiple Spin manifests"),
    }
}

// This isn't currently transitive - I don't think we have a need for that
// but could add it if we did (WAGI has code for this but it's a huge faff)
#[allow(dead_code)]
pub(crate) fn parcels_in_group(inv: &Invoice, group: &str) -> Vec<Label> {
    inv.parcel
        .as_ref()
        .unwrap_or(EMPTY)
        .iter()
        .filter_map(|p| {
            if is_member(p, group) {
                Some(p.label.clone())
            } else {
                None
            }
        })
        .collect_vec()
}

pub(crate) fn is_member(parcel: &Parcel, group: &str) -> bool {
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
    NoToken(NoToken),
}

#[async_trait]
impl TokenManager for BindleTokenManager {
    async fn apply_auth_header(&self, builder: RequestBuilder) -> client::Result<RequestBuilder> {
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
pub(crate) struct BindleReader {
    inner: BindleReaderInner,
}

impl BindleReader {
    /// Gets the content of a parcel from the bindle source.
    pub(crate) async fn get_parcel(&self, id: &str) -> Result<Vec<u8>> {
        match &self.inner {
            BindleReaderInner::Remote(c, bindle_id) => c
                .get_parcel(bindle_id, id)
                .await
                .with_context(|| anyhow!("Error fetching remote parcel {}@{}", bindle_id, id)),

            BindleReaderInner::Standalone(s) => {
                let path = s.parcel_dir.join(format!("{}.dat", id));
                fs::read(&path).await.with_context(|| {
                    anyhow!(
                        "Error reading standalone parcel {} from {}",
                        id,
                        path.display()
                    )
                })
            }
        }
    }

    /// Get the invoice from the bindle source
    pub(crate) async fn get_invoice(&self) -> Result<Invoice> {
        match &self.inner {
            BindleReaderInner::Remote(c, id) => c
                .get_invoice(id)
                .await
                .with_context(|| anyhow!("Error fetching remote invoice {}", id)),

            BindleReaderInner::Standalone(s) => {
                let bytes = fs::read(&s.invoice_file).await.with_context(|| {
                    anyhow!(
                        "Error reading bindle invoice rom '{}'",
                        s.invoice_file.display()
                    )
                })?;
                toml::from_slice(&bytes).with_context(|| {
                    anyhow!(
                        "Error parsing file '{}' as invoice",
                        s.invoice_file.display()
                    )
                })
            }
        }
    }

    pub(crate) fn remote(c: &Client<BindleTokenManager>, id: &Id) -> Self {
        Self {
            inner: BindleReaderInner::Remote(c.clone(), id.clone()),
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn standalone(base_path: impl AsRef<Path>, id: &Id) -> Result<Self> {
        let s = StandaloneRead::new(&base_path, id).await?;
        Ok(Self {
            inner: BindleReaderInner::Standalone(Arc::new(s)),
        })
    }
}

#[derive(Clone)]
enum BindleReaderInner {
    Standalone(Arc<StandaloneRead>),
    Remote(Client<BindleTokenManager>, Id),
}

impl Debug for BindleReaderInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standalone(_) => f.debug_tuple("Standalone").finish(),
            Self::Remote(_, _) => f.debug_tuple("Remote").finish(),
        }
    }
}
