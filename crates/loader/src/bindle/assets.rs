#![deny(missing_docs)]

use super::utils::BindleReader;
use crate::assets::{create_dir, ensure_under};
use anyhow::{anyhow, bail, Context, Result};
use bindle::{Id, Label};
use futures::future;
use spin_config::DirectoryMount;
use std::path::Path;
use tokio::fs;
use tracing::log;

pub(crate) async fn prepare_component(
    reader: &BindleReader,
    bindle_id: &Id,
    parcels: &[Label],
    base_dst: impl AsRef<Path>,
    component: &str,
) -> Result<DirectoryMount> {
    let copier = Copier {
        reader: reader.clone(),
        id: bindle_id.clone(),
    };
    copier.prepare(parcels, base_dst, component).await
}

pub(crate) struct Copier {
    reader: BindleReader,
    id: Id,
}

impl Copier {
    async fn prepare(
        &self,
        parcels: &[Label],
        base_dst: impl AsRef<Path>,
        component: &str,
    ) -> Result<DirectoryMount> {
        log::info!(
            "Mounting files from '{}' to '{}'",
            self.id,
            base_dst.as_ref().display()
        );

        let host = create_dir(&base_dst, component).await?;
        let guest = "/".to_string();
        self.copy_all(parcels, &host).await?;

        Ok(DirectoryMount { host, guest })
    }

    async fn copy_all(&self, parcels: &[Label], dir: impl AsRef<Path>) -> Result<()> {
        match future::join_all(parcels.iter().map(|p| self.copy(p, &dir)))
            .await
            .into_iter()
            .filter_map(|r| r.err())
            .map(|e| log::error!("{:?}", e))
            .count()
        {
            0 => Ok(()),
            n => bail!("Error copying assets: {} file(s) not copied", n),
        }
    }

    /// Copy
    async fn copy(&self, p: &Label, dir: impl AsRef<Path>) -> Result<()> {
        let to = dir.as_ref().join(&p.name);

        ensure_under(&dir, &to)?;

        log::trace!(
            "Copying asset file '{}@{}' -> '{}'",
            self.id,
            p.sha256,
            to.display()
        );
        fs::create_dir_all(to.parent().expect("Cannot copy to file '/'")).await?;
        let buf =
            self.reader.get_parcel(&p.sha256).await.with_context(|| {
                anyhow!("Failed to fetch asset parcel '{}@{}'", self.id, p.sha256)
            })?;
        fs::write(&to, &buf).await.with_context(|| {
            anyhow!(
                "Failed to write asset parcel '{}@{}' to {}",
                self.id,
                p.sha256,
                to.display()
            )
        })?;
        Ok(())
    }
}
