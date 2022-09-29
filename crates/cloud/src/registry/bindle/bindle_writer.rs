#![deny(missing_docs)]

use anyhow::{Context, Result};
use bindle::{Invoice, Parcel};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

struct BindleWriter {
    source_dir: PathBuf,
    dest_dir: PathBuf,
    invoice: Invoice,
    parcel_sources: ParcelSources,
}

/// Writes an invoice and supporting parcels out as a standalone bindle.
pub async fn write(
    source_dir: impl AsRef<Path>,
    dest_dir: impl AsRef<Path>,
    invoice: &Invoice,
    parcel_sources: &ParcelSources,
) -> Result<()> {
    let writer = BindleWriter {
        source_dir: source_dir.as_ref().to_owned(),
        dest_dir: dest_dir.as_ref().to_owned(),
        invoice: invoice.clone(),
        parcel_sources: parcel_sources.clone(),
    };
    writer.write().await
}

impl BindleWriter {
    async fn write(&self) -> Result<()> {
        // This is very similar to bindle::StandaloneWrite::write but... not quite the same
        let bindle_id_hash = self.invoice.bindle.id.sha();
        let bindle_dir = self.dest_dir.join(bindle_id_hash);
        let parcels_dir = bindle_dir.join("parcels");
        tokio::fs::create_dir_all(&parcels_dir).await?;

        self.write_invoice_file(&bindle_dir).await?;
        self.write_parcel_files(&parcels_dir).await?;
        Ok(())
    }

    async fn write_invoice_file(&self, bindle_dir: &Path) -> Result<()> {
        let invoice_text = toml::to_string_pretty(&self.invoice)?;
        let invoice_file = bindle_dir.join("invoice.toml");
        tokio::fs::write(&invoice_file, &invoice_text)
            .await
            .with_context(|| format!("Failed to write invoice to '{}'", invoice_file.display()))?;
        Ok(())
    }

    async fn write_parcel_files(&self, parcels_dir: &Path) -> Result<()> {
        let parcels = match &self.invoice.parcel {
            Some(p) => p,
            None => return Ok(()),
        };

        let parcel_writes = parcels
            .iter()
            .map(|parcel| self.write_one_parcel(parcels_dir, parcel));
        futures::future::join_all(parcel_writes)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }

    async fn write_one_parcel(&self, parcels_dir: &Path, parcel: &Parcel) -> Result<()> {
        let source_file = match self.parcel_sources.source(&parcel.label.sha256) {
            Some(path) => path.clone(),
            None => self.source_dir.join(&parcel.label.name),
        };
        let hash = &parcel.label.sha256;
        let dest_file = parcels_dir.join(format!("{}.dat", hash));
        tokio::fs::copy(&source_file, &dest_file)
            .await
            .with_context(|| copy_parcel_failed_msg(&source_file, &dest_file))?;

        if has_annotation(parcel, DELETE_ON_WRITE) {
            tokio::fs::remove_file(&source_file).await.ignore_errors(); // Leaking a temp file is sad but not a reason to fail
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ParcelSource {
    digest: String,
    source_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ParcelSources {
    sources: Vec<ParcelSource>,
}

impl ParcelSources {
    pub fn source(&self, digest: &str) -> Option<&PathBuf> {
        self.sources
            .iter()
            .find(|s| s.digest == digest)
            .map(|s| &s.source_path)
    }

    pub fn single(digest: &str, source: impl AsRef<Path>) -> Self {
        let parcel_source = ParcelSource {
            digest: digest.to_owned(),
            source_path: source.as_ref().to_owned(),
        };
        Self {
            sources: vec![parcel_source],
        }
    }

    pub fn from_iter(paths: impl Iterator<Item = (String, impl AsRef<Path>)>) -> Self {
        let sources = paths
            .map(|(digest, path)| ParcelSource {
                digest,
                source_path: path.as_ref().to_owned(),
            })
            .collect();

        Self { sources }
    }
}

fn has_annotation(parcel: &Parcel, key: &str) -> bool {
    match &parcel.label.annotations {
        Some(map) => map.contains_key(key),
        None => false,
    }
}

const DELETE_ON_WRITE: &str = "fermyon:spin:delete_on_write";

pub(crate) fn delete_after_copy() -> BTreeMap<String, String> {
    BTreeMap::from([(DELETE_ON_WRITE.to_owned(), ".".to_owned())])
}

trait IgnoreErrors {
    fn ignore_errors(&self);
}

impl<E> IgnoreErrors for Result<(), E> {
    fn ignore_errors(&self) {}
}

fn copy_parcel_failed_msg(source_file: &Path, dest_file: &Path) -> String {
    format!(
        "Failed to copy parcel from {} to '{}'",
        source_file.display(),
        dest_file.display()
    )
}
