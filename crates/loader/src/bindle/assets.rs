#![deny(missing_docs)]

use crate::{
    assets::{create_dir, ensure_under},
    bindle::utils::BindleReader,
};
use anyhow::{anyhow, bail, Context, Result};
use bindle::{Id, Label};
use futures::{future, stream, StreamExt, TryStreamExt};
use spin_manifest::DirectoryMount;
use std::path::Path;
use tokio::{fs, io::AsyncWriteExt};
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
        match stream::iter(parcels.iter().map(|p| self.copy(p, &dir)))
            .buffer_unordered(crate::MAX_PARALLEL_ASSET_PROCESSING)
            .filter_map(|r| future::ready(r.err()))
            .map(|e| log::error!("{:?}", e))
            .count()
            .await
        {
            0 => Ok(()),
            n => bail!("Error copying assets: {} file(s) not copied", n),
        }
    }

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
        let mut stream = self
            .reader
            .get_parcel_stream(&p.sha256)
            .await
            .with_context(|| anyhow!("Failed to fetch asset parcel '{}@{}'", self.id, p.sha256))?;

        let mut file = fs::File::create(&to).await.with_context(|| {
            anyhow!(
                "Failed to create local file for asset parcel '{}@{}'",
                self.id,
                p.sha256
            )
        })?;

        while let Some(chunk) = stream
            .try_next()
            .await
            .with_context(|| anyhow!("Failed to read asset parcel '{}@{}'", self.id, p.sha256))?
        {
            file.write_all(&chunk).await.with_context(|| {
                anyhow!(
                    "Failed to write asset parcel '{}@{}' to {}",
                    self.id,
                    p.sha256,
                    to.display()
                )
            })?;
        }

        Ok(())
    }
}
