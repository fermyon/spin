#![deny(missing_docs)]

use anyhow::{anyhow, bail, Context, Error, Result};
use bindle::{client::Client, standalone::StandaloneRead, Id, Invoice, Label, Parcel};
use futures::{Stream, StreamExt, TryStreamExt};
use itertools::Itertools;
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{fs, io::AsyncWriteExt};
use tokio_util::codec::{BytesCodec, FramedRead};

use super::connection::AnyAuth;

static EMPTY: &Vec<bindle::Parcel> = &vec![];

// Alternative to storing `spin.toml` as a parcel, this could be
// distinguished it through a group, or an annotation.

/// The media type of a `spin.toml` parcel as part of a bindle.
pub const SPIN_MANIFEST_MEDIA_TYPE: &str = "application/vnd.fermyon.spin+toml";

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
                let path = s.parcel_data_path(id);
                fs::read(&path).await.with_context(|| {
                    anyhow!(
                        "Error reading standalone parcel {} from {}",
                        id,
                        path.display()
                    )
                })
            }

            BindleReaderInner::Caching(s, c, bindle_id) => {
                let path = s.parcel_data_path(id);
                match fs::read(&path).await {
                    Ok(contents) => Ok(contents),
                    Err(_) => {
                        copy_remote_parcel(c, bindle_id, id, &path).await?;
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
        }
    }

    /// Gets the content of a parcel from the bindle source as a stream.
    pub(crate) async fn get_parcel_stream(
        &self,
        id: &str,
    ) -> Result<impl Stream<Item = Result<bytes::Bytes>> + '_> {
        match &self.inner {
            BindleReaderInner::Remote(c, bindle_id) => c
                .get_parcel_stream(bindle_id, id)
                .await
                .with_context(|| anyhow!("Error fetching remote parcel {}@{}", bindle_id, id))
                .map(|s| s.map_err(Error::from).boxed()),

            BindleReaderInner::Standalone(s) => {
                let path = s.parcel_data_path(id);
                let file = fs::File::open(&path).await.with_context(|| {
                    anyhow!(
                        "Error reading standalone parcel {} from {}",
                        id,
                        path.display()
                    )
                })?;
                Ok(FramedRead::new(file, BytesCodec::new())
                    .map_ok(bytes::BytesMut::freeze)
                    .map_err(Error::from)
                    .boxed())
            }

            BindleReaderInner::Caching(s, c, bindle_id) => {
                let path = s.parcel_data_path(id);
                match fs::File::open(&path).await {
                    Ok(file) => Ok(FramedRead::new(file, BytesCodec::new())
                        .map_ok(bytes::BytesMut::freeze)
                        .map_err(Error::from)
                        .boxed()),
                    Err(_) => {
                        copy_remote_parcel(c, bindle_id, id, &path).await?;
                        let file = fs::File::open(&path).await.with_context(|| {
                            anyhow!(
                                "Error reading standalone parcel {} from {}",
                                id,
                                path.display()
                            )
                        })?;
                        Ok(FramedRead::new(file, BytesCodec::new())
                            .map_ok(bytes::BytesMut::freeze)
                            .map_err(Error::from)
                            .boxed())
                    }
                }
            }
        }
    }

    /// Get the invoice from the bindle source
    pub(crate) async fn get_invoice(&self) -> Result<Invoice> {
        match &self.inner {
            BindleReaderInner::Remote(c, id) => get_invoice_from_remote(c, id).await,

            BindleReaderInner::Standalone(s) => get_invoice_from_standalone(&s.invoice_file).await,

            BindleReaderInner::Caching(s, c, id) => {
                match get_invoice_from_standalone(&s.invoice_file).await {
                    Ok(inv) => Ok(inv),
                    Err(_) => backfill_invoice(s, c, id).await,
                }
            }
        }
    }

    pub(crate) async fn remote(
        c: &Client<AnyAuth>,
        id: &Id,
        cache_dir: &Option<PathBuf>,
    ) -> Result<Self> {
        match cache_dir {
            None => Ok(Self::remote_only(c, id)),
            Some(path) => Self::caching(c, id, path).await,
        }
    }

    pub(crate) fn remote_only(c: &Client<AnyAuth>, id: &Id) -> Self {
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

    pub(crate) async fn caching(
        c: &Client<AnyAuth>,
        bindle_id: &Id,
        cache_dir: &Path,
    ) -> Result<Self> {
        tokio::fs::create_dir_all(cache_dir)
            .await
            .with_context(|| format!("Failed to create cache dir {}", cache_dir.display()))?;
        let s = StandaloneLayout::new(cache_dir, bindle_id).await?;
        Ok(Self {
            inner: BindleReaderInner::Caching(Arc::new(s), c.clone(), bindle_id.clone()),
        })
    }
}

struct StandaloneLayout {
    invoice_file: PathBuf,
    parcel_dir: PathBuf,
}

impl StandaloneLayout {
    pub async fn new(base_path: &Path, id: &Id) -> anyhow::Result<Self> {
        let path = base_path.join(id.sha());
        tokio::fs::create_dir_all(&path).await.with_context(|| {
            format!(
                "Failed to create cache directory {} for {}",
                path.display(),
                id
            )
        })?;
        let invoice_file = path.join(bindle::standalone::INVOICE_FILE);
        let parcel_dir = path.join(bindle::standalone::PARCEL_DIR);
        Ok(Self {
            invoice_file,
            parcel_dir,
        })
    }

    pub fn parcel_data_path(&self, parcel_id: &str) -> PathBuf {
        self.parcel_dir.join(format!("{}.dat", parcel_id))
    }
}

async fn get_invoice_from_remote(c: &Client<AnyAuth>, id: &Id) -> Result<Invoice, Error> {
    c.get_invoice(id)
        .await
        .with_context(|| anyhow!("Error fetching remote invoice {}", id))
}

async fn get_invoice_from_standalone(invoice_file: &Path) -> Result<Invoice, Error> {
    let bytes = fs::read(invoice_file).await.with_context(|| {
        anyhow!(
            "Error reading bindle invoice rom '{}'",
            invoice_file.display()
        )
    })?;
    toml::from_slice(&bytes)
        .with_context(|| anyhow!("Error parsing file '{}' as invoice", invoice_file.display()))
}

async fn backfill_invoice(
    s: &Arc<StandaloneLayout>,
    c: &Client<AnyAuth>,
    id: &Id,
) -> Result<Invoice> {
    let invoice = get_invoice_from_remote(c, id).await?;
    write_invoice_file(s, &invoice).await?; // TODO: Should we ignore this?
    Ok(invoice)
}

async fn write_invoice_file(s: &Arc<StandaloneLayout>, invoice: &Invoice) -> Result<()> {
    let invoice_text = toml::to_string_pretty(invoice)?;
    let invoice_file = &s.invoice_file;
    tokio::fs::write(invoice_file, &invoice_text)
        .await
        .with_context(|| format!("Failed to cache invoice to '{}'", invoice_file.display()))?;
    Ok(())
}

#[derive(Clone)]
enum BindleReaderInner {
    Standalone(Arc<StandaloneRead>),
    Remote(Client<AnyAuth>, Id),
    Caching(Arc<StandaloneLayout>, Client<AnyAuth>, Id),
}

impl Debug for BindleReaderInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standalone(_) => f.debug_tuple("Standalone").finish(),
            Self::Remote(_, _) => f.debug_tuple("Remote").finish(),
            Self::Caching(..) => f.debug_tuple("Caching").finish(),
        }
    }
}

pub(crate) async fn copy_remote_parcel(
    client: &Client<AnyAuth>,
    bindle_id: &Id,
    sha: &str,
    to: &Path,
) -> anyhow::Result<()> {
    let mut stream = client
        .get_parcel_stream(bindle_id, sha)
        .await
        .with_context(|| anyhow!("Failed to fetch asset parcel '{}@{}'", bindle_id, sha))?;

    fs::create_dir_all(to.parent().expect("Cannot copy to file '/'")).await?;

    let mut file = fs::File::create(to).await.with_context(|| {
        anyhow!(
            "Failed to create local file for asset parcel '{}@{}'",
            bindle_id,
            sha
        )
    })?;

    while let Some(chunk) = stream
        .try_next()
        .await
        .with_context(|| anyhow!("Failed to read asset parcel '{}@{}'", bindle_id, sha))?
    {
        file.write_all(&chunk).await.with_context(|| {
            anyhow!(
                "Failed to write asset parcel '{}@{}' to {}",
                bindle_id,
                sha,
                to.display()
            )
        })?;
    }

    Ok(())
}
