use std::{collections::HashMap, sync::{Arc, RwLock}, path::{Path, PathBuf}};

use anyhow::{bail, Context};
use spin_http_engine::{HttpTrigger, TlsConfig, HttpTriggerExecutionConfig};
use spin_manifest::ApplicationTrigger;
use spin_redis_engine::RedisTrigger;
use spin_trigger::{ExecutionOptions, run_trigger};
use tempfile::TempDir;

use crate::{schema::{WorkloadId, WorkloadManifest}, store::WorkStore, WorkloadSpec};

#[async_trait::async_trait]
pub(crate) trait Scheduler {
    async fn notify_changed(&self, workload: &WorkloadId) -> anyhow::Result<()>;
}

pub(crate) struct LocalScheduler {
    store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
    running: Arc<RwLock<HashMap<WorkloadId, RunningWorkload>>>,
}

impl LocalScheduler {
    pub(crate) fn new(store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>) -> Self {
        Self {
            store,
            running: Arc::new(RwLock::new(HashMap::new()))
        }
    }
}

struct RunningWorkload {
    work_dir: WorkingDirectory,
    handle: RunHandle,
}

enum RunHandle {
    Fut(tokio::task::JoinHandle<anyhow::Result<()>>),
}

impl LocalScheduler {
    pub async fn notify_changed(&self, workload: &WorkloadId) -> anyhow::Result<()> {
        // TODO: look at WorkloadSpec::status
        match self.extricate(workload) {
            (Some(w), Some(c)) => self.restart_workload(workload, w, c).await?,
            (Some(w), None) => self.start_workload(workload, w).await?,
            (None, Some(c)) => self.stop_workload(workload, c),
            (None, None) => (),
        }

        Ok(())
    }

    fn extricate(&self, workload: &WorkloadId) -> (Option<WorkloadSpec>, Option<RunningWorkload>) {
        let spec = self.store.read().unwrap().get_workload(workload);
        let mut running = self.running.write().unwrap();
        let current = running.remove(workload);
        (spec, current)
    }

    async fn start_workload(&self, workload: &WorkloadId, spec: WorkloadSpec) -> anyhow::Result<()> {
        // Identify the application type
        // Instantiate the relevant trigger
        // Start the relevant trigger
        let running = self.start_workload_from_spec(spec).await?;
        // Stash the task
        self.running.write().unwrap().insert(workload.clone(), running);
        Ok(())
    }

    async fn start_workload_from_spec(&self, spec: WorkloadSpec) -> anyhow::Result<RunningWorkload> {
        let working_dir_holder = match &spec.opts.tmp {
            None => WorkingDirectory::Temporary(tempfile::tempdir()?),
            Some(d) => WorkingDirectory::Given(d.to_owned()),
        };
        let working_dir = working_dir_holder.path();

        let mut app = match &spec.manifest {
            WorkloadManifest::File(manifest_file) => {
                let bindle_connection = spec.opts.bindle_connection();
                spin_loader::from_file(manifest_file, working_dir, &bindle_connection).await?
            },
            WorkloadManifest::Bindle(bindle) => match &spec.opts.server {
                Some(server) => spin_loader::from_bindle(bindle, server, working_dir).await?,
                _ => bail!("Loading from a bindle requires a Bindle server URL"),
            },
        };
        append_env(&mut app, &spec.opts.env)?;

        if let Some(ref mut resolver) = app.config_resolver {
            // TODO(lann): This should be safe but ideally this get_mut would be refactored away.
            let resolver = Arc::get_mut(resolver)
                .context("Internal error: app.config_resolver unexpectedly shared")?;
            // TODO(lann): Make config provider(s) configurable.
            resolver.add_provider(spin_config::provider::env::EnvProvider::default());
        }

        let tls = match (spec.opts.tls_key.clone(), spec.opts.tls_cert.clone()) {
            (Some(key_path), Some(cert_path)) => {
                if !cert_path.is_file() {
                    bail!("TLS certificate file does not exist or is not a file")
                }
                if !key_path.is_file() {
                    bail!("TLS key file does not exist or is not a file")
                }
                Some(TlsConfig {
                    cert_path,
                    key_path,
                })
            }
            (None, None) => None,
            _ => unreachable!(),
        };

        let wasmtime_config = spec.opts.wasmtime_default_config()?;

        let follow = spec.opts.follow_components();

        let jh = match &app.info.trigger {
            ApplicationTrigger::Http(_) => {
                tokio::spawn(run_trigger(
                    app,
                    ExecutionOptions::<HttpTrigger>::new(
                        spec.opts.log.clone(),
                        follow,
                        HttpTriggerExecutionConfig::new(spec.opts.address, tls),
                    ),
                    Some(wasmtime_config),
                ))
            }
            ApplicationTrigger::Redis(_) => {
                tokio::spawn(run_trigger(
                    app,
                    ExecutionOptions::<RedisTrigger>::new(spec.opts.log.clone(), follow, ()),
                    Some(wasmtime_config),
                ))
            }
        };

        Ok(RunningWorkload {
            work_dir: working_dir_holder,
            handle: RunHandle::Fut(jh),
        })
    }

    async fn restart_workload(&self, workload: &WorkloadId, spec: WorkloadSpec, current: RunningWorkload) -> anyhow::Result<()> {
        self.stop_workload(workload, current);
        self.start_workload(workload, spec).await
    }

    fn stop_workload(&self, workload: &WorkloadId, current: RunningWorkload) {
        current.stop();
        self.running.write().unwrap().remove(workload);
    }
}

impl RunningWorkload {
    fn stop(self) {
        match self.handle {
            RunHandle::Fut(f) => f.abort(),
        }
        drop(self.work_dir);
    }
}

enum WorkingDirectory {
    Given(PathBuf),
    Temporary(TempDir),
}

impl WorkingDirectory {
    fn path(&self) -> &Path {
        match self {
            Self::Given(p) => p,
            Self::Temporary(t) => t.path(),
        }
    }
}

fn append_env(app: &mut spin_manifest::Application, env: &[(String, String)]) -> anyhow::Result<()> {
    for c in app.components.iter_mut() {
        for (k, v) in env {
            c.wasm.environment.insert(k.clone(), v.clone());
        }
    }
    Ok(())
}
