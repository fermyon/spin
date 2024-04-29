use command_group::AsyncCommandGroup;
use std::path::PathBuf;
use uuid::Uuid;

use super::uppificator::Pause;
#[derive(Debug)]
pub(crate) struct Buildifier {
    pub spin_bin: PathBuf,
    pub manifest: PathBuf,
    pub clear_screen: bool,
    pub has_ever_built: bool,
    pub watched_changes: tokio::sync::watch::Receiver<(Uuid, String)>, // TODO: refine which component(s) a change affects
    pub uppificator_pauser: tokio::sync::mpsc::Sender<Pause>,
}

impl Buildifier {
    #[allow(clippy::collapsible_if)]
    pub(crate) async fn run(&mut self) {
        // Other components may close channels as part of shutdown, so if any channels
        // fail, just exit the loop and fall out normally.

        loop {
            if self.clear_screen {
                _ = clearscreen::clear();
            }

            if self.uppificator_pauser.send(Pause::Pause).await.is_err() {
                break;
            }

            let (_, ref changed_paths) = self.watched_changes.borrow_and_update().clone();
            tracing::debug!("Detected changes in: {}", changed_paths);

            let build_component_result = self.build_component(changed_paths).await;

            if !self.has_ever_built {
                self.has_ever_built = matches!(build_component_result, Ok(true));
            }

            if self.has_ever_built {
                if self.uppificator_pauser.send(Pause::Unpause).await.is_err() {
                    break;
                }
            }

            if self.watched_changes.changed().await.is_err() {
                break;
            }
        }
    }

    pub(crate) async fn build_component(
        &mut self,
        mut component_path: &str,
    ) -> std::io::Result<bool> {
        let manifest = spin_manifest::manifest_from_file(&self.manifest).unwrap();
        let inner_ids: Vec<&str> = manifest.components.keys().map(|key| key.as_ref()).collect();

        if !inner_ids.iter().any(|id| component_path.contains(id)) && !component_path.is_empty() {
            component_path = inner_ids.first().cloned().unwrap_or_default();
        }

        for inner_id in inner_ids {
            if component_path.contains(inner_id) {
                component_path = inner_id;
                break;
            }
        }

        loop {
            let mut cmd = tokio::process::Command::new(&self.spin_bin);

            if component_path.is_empty() {
                cmd.arg("build").arg("-f").arg(&self.manifest);
            } else {
                cmd.arg("build")
                    .arg("-c")
                    .arg(component_path)
                    .arg("-f")
                    .arg(&self.manifest);
            }

            let mut child = cmd.group_spawn()?;

            tokio::select! {
                    exit_status = child.wait() => {
                        // It reports its own errors so we only care about success or failure (and then only for
                        // the initial build).
                        return Ok(exit_status?.success());
                    }
                    _ = self.watched_changes.changed() => {
                        tracing::debug!("Cancelling build as there are new changes to process");
                        child.kill()?;
                        if self.clear_screen {
                            _ = clearscreen::clear();
                        }
                        continue;
                    }
            }
        }
    }
}
