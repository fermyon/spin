use std::path::PathBuf;

use anyhow::{Context, Error};
use spin_controller::{RemoteCommandSender, CommandSender, RemoteEventReceiver, WorkloadEventReceiver, ControllerCommand, WorkloadId, WorkloadSpec};

const EVT_ADDR: &str = "127.0.0.1:2626";

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(atty::is(atty::Stream::Stderr))
        .init();

    let mut port_dispenser = PortDispenser::new();

    let (evt_handler, evt_listener) = message_io::node::split();
    evt_handler.network().listen(message_io::network::Transport::FramedTcp, EVT_ADDR).unwrap();
    let evt_rx = RemoteEventReceiver::new(evt_handler, evt_listener);
    let mut work_rx = WorkloadEventReceiver::Remote(evt_rx);

    let cmd_sender = RemoteCommandSender::new("127.0.0.1:4646");
    let cmd_tx = CommandSender::Remote(cmd_sender);

    cmd_tx.send(ControllerCommand::Connect(EVT_ADDR.to_owned()))
        .context("Connecting to controller")?;

    println!("Connected to controller");

    let (key_tx, mut key_rx) = tokio::sync::broadcast::channel(1);
    let keyh = tokio::task::spawn(async move {
        loop {
            let mut s = "".to_owned();
            let _ = std::io::stdin().read_line(&mut s);
            if let Some((c, arg)) = s.trim().split_once(' ') {
                match c {
                    "new" => { let _ = key_tx.send(OperatorCommand::New(arg.to_owned())); },
                    "stop" => { let _ = key_tx.send(OperatorCommand::Stop(arg.to_owned())); },
                    _ => (),
                }
            } else {
                match s.trim() {
                    "q" => { let _ = key_tx.send(OperatorCommand::Quit); break; },
                    _ => (),
                }
            }
        }
    });

    loop {
        match wait_next(
            &cmd_tx,
            &mut work_rx,
            &mut key_rx,
            &mut port_dispenser,
        ).await? {
            true => {
                // println!("loop requested continuation");
            },
            false => {
                println!("Exiting");
                break;
            }
        }
    }

    keyh.abort();

    Ok(())
}

async fn wait_next(
    cmd_tx: &CommandSender,
    work_rx: &mut WorkloadEventReceiver,
    key_rx: &mut tokio::sync::broadcast::Receiver<OperatorCommand>,
    port_dispenser: &mut PortDispenser,
) -> anyhow::Result<bool> {
    tokio::select! {
        msg = work_rx.recv() => {
            match msg {
                Ok(spin_controller::WorkloadEvent::Stopped(id, err)) => {
                    match err {
                        None => {
                            println!("Application {} stopped without error", id);
                            Ok(true)
                        },
                        Some(e) => {
                            println!("Application {} stopped with error! {}", id, e);
                            Ok(true)
                        }
                    }
                },
                Ok(spin_controller::WorkloadEvent::UpdateFailed(id, err)) => {
                    println!("Application {} failed to start with error {}", id, err);
                    Ok(true)
                },
                Err(e) => anyhow::bail!(anyhow::Error::from(e).context("Error receiving notification from controller")),
            }
        },
        cmd = key_rx.recv() => {
            match cmd {
                Ok(OperatorCommand::Stop(id)) => {
                    cmd_tx.send(ControllerCommand::RemoveWorkload(WorkloadId::new_from(&id)))?;
                    Ok(true)
                },
                Ok(OperatorCommand::Quit) => {
                    cmd_tx.send(ControllerCommand::Shutdown)?;
                    Ok(false)
                },
                Ok(OperatorCommand::New(path)) => {
                    let id = WorkloadId::new();
                    let port = port_dispenser.dispense();
                    let address = format!("127.0.0.1:{}", port);
                    let new_spec = WorkloadSpec {
                        status: spin_controller::WorkloadStatus::Running,
                        opts: spin_controller::WorkloadOpts {
                            server: None,
                            address: address.clone(),
                            tmp: None,
                            env: vec![],
                            tls_cert: None,
                            tls_key: None,
                            log: None,
                            disable_cache: false,
                            cache: None,
                            follow_components: vec![],
                            follow_all_components: true,
                        },
                        manifest: spin_controller::WorkloadManifest::File(PathBuf::from(path)),
                    };
                    cmd_tx.send(ControllerCommand::SetWorkload(id.clone(), new_spec))?;
                    println!("id: {}", id);
                    println!("addr: {}", address);
                    Ok(true)
                },
                Err(e) => anyhow::bail!(anyhow::Error::from(e).context("Error receiving command from stdin")),
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum OperatorCommand {
    // Remove(String),
    Stop(String),
    Quit,
    New(String),
}

struct PortDispenser {
    next: u16,
}

impl PortDispenser {
    pub fn new() -> Self {
        Self { next: 3010 }
    }

    pub fn dispense(&mut self) -> u16 {
        self.next = self.next + 1;
        self.next
    }
}
