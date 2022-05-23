// This is where the old Up logic goes

use std::sync::Arc;

use anyhow::{bail, Context};
use spin_http_engine::{HttpTrigger, TlsConfig, HttpTriggerExecutionConfig};
use spin_manifest::ApplicationTrigger;
use spin_redis_engine::RedisTrigger;
use spin_trigger::{ExecutionOptions, run_trigger};

use crate::{schema::{WorkloadId, WorkloadManifest}, WorkloadSpec, WorkloadEvent, scheduler::{WorkingDirectory, RunningWorkload, RunHandle}};

pub(crate) async fn run(workload: &WorkloadId, spec: WorkloadSpec, notification_sender: &tokio::sync::broadcast::Sender<WorkloadEvent>) -> anyhow::Result<RunningWorkload> {
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

    let tx = notification_sender.clone();
    let id = workload.clone();

    let jh = match &app.info.trigger {
        ApplicationTrigger::Http(_) => {
            tokio::spawn(async move {
                let r = run_trigger(
                    app,
                    ExecutionOptions::<HttpTrigger>::new(
                        spec.opts.log.clone(),
                        follow,
                        HttpTriggerExecutionConfig::new(spec.opts.address, tls),
                    ),
                    Some(wasmtime_config),
                ).await;
                let err = match r {
                    Ok(()) => None,
                    Err(e) => Some(Arc::new(e)),
                };
                // TODO: this should update the workflow status in the scheduler's record
                let _ = tx.send(WorkloadEvent::Stopped(id, err));
            })
        }
        ApplicationTrigger::Redis(_) => {
            tokio::spawn(async move {
                let r = run_trigger(
                    app,
                    ExecutionOptions::<RedisTrigger>::new(spec.opts.log.clone(), follow, ()),
                    Some(wasmtime_config),
                ).await;
                let err = match r {
                    Ok(()) => None,
                    Err(e) => Some(Arc::new(e)),
                };
                let _ = tx.send(WorkloadEvent::Stopped(id, err));
            })
        }
    };

    Ok(RunningWorkload {
        work_dir: working_dir_holder,
        handle: RunHandle::Fut(jh),
    })
}

fn append_env(app: &mut spin_manifest::Application, env: &[(String, String)]) -> anyhow::Result<()> {
    for c in app.components.iter_mut() {
        for (k, v) in env {
            c.wasm.environment.insert(k.clone(), v.clone());
        }
    }
    Ok(())
}
