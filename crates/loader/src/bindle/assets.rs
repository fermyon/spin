#![deny(missing_docs)]

use std::path::Path;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use bindle::{Id, Label};
use futures::{future, stream, StreamExt, TryStreamExt};
use spin_manifest::DirectoryMount;
use tokio::{fs, io::AsyncWriteExt};
use tracing::log;

use crate::digest::file_sha256_string;
use crate::{
    assets::{create_dir, ensure_under},
    bindle::utils::BindleReader,
    local::parent_dir,
};

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

        if to.exists() {
            match check_existing_file(to.clone(), p).await {
                // Copy already exists
                Ok(true) => return Ok(()),
                Ok(false) => (),
                Err(err) => tracing::error!("Error verifying existing parcel: {}", err),
            }
        }

        log::trace!(
            "Copying asset file '{}@{}' -> '{}'",
            self.id,
            p.sha256,
            to.display()
        );
        fs::create_dir_all(parent_dir(&to).expect("Cannot copy to file '/'")).await?;
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

async fn check_existing_file(path: PathBuf, label: &Label) -> Result<bool> {
    let sha256_digest = tokio::task::spawn_blocking(move || file_sha256_string(path)).await??;
    Ok(sha256_digest == label.sha256)
}
