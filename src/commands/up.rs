use std::path::{PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser};
use spin_controller::{WorkloadEvent, Control, WorkloadId, WorkloadSpec};

use crate::opts::*;

/// Start the Fermyon runtime.
#[derive(Parser, Debug)]
#[clap(about = "Start the Spin application")]

pub struct UpCommand {
    #[structopt(flatten)]
    pub opts: UpOpts,

    /// Path to spin.toml.
    #[clap(
            name = APP_CONFIG_FILE_OPT,
            short = 'f',
            long = "file",
            conflicts_with = BINDLE_ID_OPT,
        )]
    pub app: Option<PathBuf>,

    /// ID of application bindle.
    #[clap(
            name = BINDLE_ID_OPT,
            short = 'b',
            long = "bindle",
            conflicts_with = APP_CONFIG_FILE_OPT,
            requires = BINDLE_SERVER_URL_OPT,
        )]
    pub bindle: Option<String>,
    /// URL of bindle server.
    #[clap(
            name = BINDLE_SERVER_URL_OPT,
            long = "bindle-server",
            env = BINDLE_URL_ENV,
        )]
    pub server: Option<String>,
}

#[derive(Args, Debug)]
pub struct UpOpts {
    /// IP address and port to listen on
    #[clap(name = ADDRESS_OPT, long = "listen", default_value = "127.0.0.1:3000")]
    pub address: String,
    /// Temporary directory for the static assets of the components.
    #[clap(long = "temp")]
    pub tmp: Option<PathBuf>,
    /// Pass an environment variable (key=value) to all components of the application.
    #[clap(long = "env", short = 'e', parse(try_from_str = crate::parse_env_var))]
    pub env: Vec<(String, String)>,

    /// The path to the certificate to use for https, if this is not set, normal http will be used. The cert should be in PEM format
    #[clap(
            name = TLS_CERT_FILE_OPT,
            long = "tls-cert",
            env = TLS_CERT_ENV_VAR,
            requires = TLS_KEY_FILE_OPT,
        )]
    pub tls_cert: Option<PathBuf>,

    /// The path to the certificate key to use for https, if this is not set, normal http will be used. The key should be in PKCS#8 format
    #[clap(
            name = TLS_KEY_FILE_OPT,
            long = "tls-key",
            env = TLS_KEY_ENV_VAR,
            requires = TLS_CERT_FILE_OPT,
        )]
    pub tls_key: Option<PathBuf>,
    /// Log directory for the stdout and stderr of components.
    #[clap(
            name = APP_LOG_DIR,
            short = 'L',
            long = "log-dir",
            )]
    pub log: Option<PathBuf>,

    /// Disable Wasmtime cache.
    #[clap(
        name = DISABLE_WASMTIME_CACHE,
        long = "disable-cache",
        env = DISABLE_WASMTIME_CACHE,
        conflicts_with = WASMTIME_CACHE_FILE,
        takes_value = false,
    )]
    pub disable_cache: bool,

    /// Wasmtime cache configuration file.
    #[clap(
        name = WASMTIME_CACHE_FILE,
        long = "cache",
        env = WASMTIME_CACHE_FILE,
        conflicts_with = DISABLE_WASMTIME_CACHE,
    )]
    pub cache: Option<PathBuf>,

    /// Print output for given component(s) to stdout/stderr
    #[clap(
        name = FOLLOW_LOG_OPT,
        long = "follow",
        multiple_occurrences = true,
        )]
    pub follow_components: Vec<String>,

    /// Print all component output to stdout/stderr
    #[clap(
        long = "follow-all",
        conflicts_with = FOLLOW_LOG_OPT,
        )]
    pub follow_all_components: bool,
}

impl UpCommand {
    pub async fn run(self) -> Result<()> {
        let mut controller = spin_controller::Control::in_memory_rpc("127.0.0.1:3636");

        let manifest = match (&self.app, &self.bindle) {
            (app, None) => {
                let manifest_file = app
                    .as_deref()
                    .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
                spin_controller::WorkloadManifest::File(manifest_file.to_owned())
            },
            (None, Some(id)) => {
                spin_controller::WorkloadManifest::Bindle(id.clone())
            },
            (Some(_), Some(_)) => bail!("Specify only one of app file or bindle ID"),
        };

        let opts = spin_controller::WorkloadOpts {
            server: self.server.clone(),
            address: self.opts.address.clone(),
            tmp: self.opts.tmp.clone(),
            env: self.opts.env.clone(),
            tls_cert: self.opts.tls_cert.clone(),
            tls_key: self.opts.tls_key.clone(),
            log: self.opts.log.clone(),
            disable_cache: self.opts.disable_cache,
            cache: self.opts.cache.clone(),
            follow_components: self.opts.follow_components.clone(),
            follow_all_components: self.opts.follow_all_components,
        };

        let the_id = spin_controller::WorkloadId::new();
        let mut spec = spin_controller::WorkloadSpec {
            status: spin_controller::WorkloadStatus::Running,
            opts,
            manifest,
        };

        let (ctrlc_tx, mut ctrlc_rx) = tokio::sync::broadcast::channel(1);
        let (key_tx, mut key_rx) = tokio::sync::broadcast::channel(1);
        let mut work_rx = controller.notifications();

        // let ctrlc_rx_recv = ctrlc_rx.recv();
        // let key_rx_recv = key_rx.recv();
        // let work_rx_recv = work_rx.recv();

        ctrlc::set_handler(move || {
            let _ = ctrlc_tx.send(());
        })?;

        controller.set_workload(&the_id, spec.clone())?;

        // TODO: this fouls up Ctrl+C handling but interesting to play with it
        // let keyh = tokio::task::spawn(async move {
        //     loop {
        //         let mut s = "".to_owned();
        //         let _ = std::io::stdin().read_line(&mut s);
        //         match s.trim() {
        //             "n" => { let _ = key_tx.send(OperatorCommand::New); },
        //             "s" => { let _ = key_tx.send(OperatorCommand::Stop); },
        //             "r" => { let _ = key_tx.send(OperatorCommand::Remove); },
        //             "q" => { let _ = key_tx.send(OperatorCommand::Quit); break; },
        //             _ => (),
        //         }
        //     }
        // });

        loop {
            match self.wait_next(
                &mut controller,
                &the_id,
                &mut spec,
                &mut ctrlc_rx,
                &mut work_rx,
                &mut key_rx,
            ).await? {
                true => {
                    // println!("loop requested continuation");
                },
                false => {
                    // println!("loop requested exit");
                    break;
                }
            }
        }

        // keyh.abort();

        Ok(())
    }

    async fn wait_next(&self,
        controller: &mut Control,
        the_id: &WorkloadId,
        spec: &mut WorkloadSpec,
        ctrlc_rx: &mut tokio::sync::broadcast::Receiver<()>,
        work_rx: &mut tokio::sync::broadcast::Receiver<WorkloadEvent>,
        key_rx: &mut tokio::sync::broadcast::Receiver<OperatorCommand>,
    ) -> anyhow::Result<bool> {
        tokio::select! {
            _ = ctrlc_rx.recv() => {
                controller.remove_workload(&the_id)?;
                Ok(false)
            },
            msg = work_rx.recv() => {
                match msg {
                    Ok(spin_controller::WorkloadEvent::Stopped(id, err)) => {
                        if &id == the_id {
                            match err {
                                None => {
                                    println!("Listener stopped without error");
                                    Ok(false)
                                },
                                Some(e) => {
                                    let err_text = format!("Listener stopped with error {:#}", e);  // because I haven't figured out how to get the error itself
                                    anyhow::bail!(err_text);
                                }
                            }
                        } else {
                            Ok(true)
                        }
                    },
                    Ok(spin_controller::WorkloadEvent::UpdateFailed(id, err)) => {
                        if &id == the_id {
                            let err_text = format!("Failed to start app with error {:#}", err);  // because I haven't figured out how to get the error itself
                            anyhow::bail!(err_text);
                        } else {
                            Ok(true)
                        }
                    },
                    Err(e) => anyhow::bail!(anyhow::Error::from(e).context("Error receiving notification from controller")),
                }
            },
            cmd = key_rx.recv() => {
                match cmd {
                    Ok(OperatorCommand::Remove) => {
                        println!("removing");
                        controller.remove_workload(&the_id)?;
                        Ok(true)
                    },
                    Ok(OperatorCommand::Stop) => {
                        println!("stopping");
                        spec.status = spin_controller::WorkloadStatus::Stopped;
                        controller.set_workload(&the_id, spec.clone())?;
                        Ok(true)
                    },
                    Ok(OperatorCommand::Quit) => {
                        println!("quitting");
                        let _ = controller.shutdown().await;
                        return Ok(false);
                    },
                    Ok(OperatorCommand::New) => {
                        let new_id = WorkloadId::new();
                        let new_spec = WorkloadSpec {
                            status: spin_controller::WorkloadStatus::Running,
                            opts: spin_controller::WorkloadOpts {
                                server: self.server.clone(),
                                address: "127.0.0.1:3001".to_owned(),
                                tmp: self.opts.tmp.clone(),
                                env: self.opts.env.clone(),
                                tls_cert: self.opts.tls_cert.clone(),
                                tls_key: self.opts.tls_key.clone(),
                                log: self.opts.log.clone(),
                                disable_cache: self.opts.disable_cache,
                                cache: self.opts.cache.clone(),
                                follow_components: self.opts.follow_components.clone(),
                                follow_all_components: self.opts.follow_all_components,
                            },
                            manifest: spin_controller::WorkloadManifest::File(PathBuf::from("./examples/wagi-http-rust/spin.toml")),
                        };
                        let _ = controller.set_workload(&new_id, new_spec);
                        Ok(true)
                    },
                    Err(e) => anyhow::bail!(anyhow::Error::from(e).context("Error receiving command from stdin")),
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum OperatorCommand {
    Remove,
    Stop,
    Quit,
    New,
}
