#![deny(missing_docs)]

use std::path::Path;

use anyhow::{Context, Result};
use bindle::{Id, Label};
use spin_config::BindleReader;
use tracing::log;

use super::*;

pub(crate) async fn prepare(
    reader: &BindleReader,
    invoice_id: &Id,
    parcels: &[Label],
    destination_base_directory: impl AsRef<Path>,
    component_id: &str,
) -> Result<DirectoryMount> {
    let copier = BindleAssetCopier {
        reader: reader.clone(),
        invoice_id: invoice_id.clone(),
    };
    copier.prepare_assets_from_bindle(
        parcels,
        destination_base_directory,
        component_id
    ).await
}

struct BindleAssetCopier {
    reader: BindleReader,
    invoice_id: Id,
}

impl BindleAssetCopier {
    async fn prepare_assets_from_bindle(
        &self,
        parcels: &[Label],
        destination_base_directory: impl AsRef<Path>,
        component_id: &str,
    ) -> Result<DirectoryMount> {
        log::info!(
            "Mounting files from '{}' to '{}'",
            self.invoice_id,
            destination_base_directory.as_ref().display()
        );

        let asset_directory =
            create_asset_directory(&destination_base_directory, component_id).await?;
        self.copy_all(parcels, &asset_directory).await?;

        Ok(DirectoryMount {
            host: asset_directory,
            guest: "/".to_string(),
        })
    }

    async fn copy_all(
        &self,
        parcels: &[bindle::Label],
        mount_directory: impl AsRef<Path>,
    ) -> Result<()> {
        let futures = parcels
            .iter()
            .map(|p| self.copy_one(p, &mount_directory));
        let results = futures::future::join_all(futures).await;
        let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
        for e in &errors {
            log::error!("{}", e);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Error copying assets: {} parcel(s) not copied", errors.len()))
        }
    }

    async fn copy_one(
        &self,
        parcel_to_mount: &bindle::Label,
        mount_directory: impl AsRef<Path>,
    ) -> Result<()> {
        let to = mount_directory.as_ref().join(&parcel_to_mount.name);

        ensure_under(&mount_directory, &to)?;

        log::trace!("Copying asset file '{}@{}' -> '{}'", self.invoice_id, parcel_to_mount.sha256, to.display());
        tokio::fs::create_dir_all(to.parent().expect("Cannot copy to file '/'")).await?;
        let parcel_content = self
            .reader
            .get_parcel(&parcel_to_mount.sha256)
            .await
            .with_context(|| format!("Failed to fetch asset parcel '{}@{}'", self.invoice_id, parcel_to_mount.sha256))?;
        tokio::fs::write(&to, &parcel_content)
            .await
            .with_context(|| format!("Failed to write asset parcel '{}@{}' to {}", self.invoice_id, parcel_to_mount.sha256, to.display()))?;
        Ok(())
    }
}
