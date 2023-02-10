use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Parser;
use glob::Pattern;
use nix::sys::signal::{self, Signal};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use spin_build::manifest::BuildAppInfoAnyVersion;
use spin_loader::local::parent_dir;
use subprocess::{Exec, Redirection};
use tokio::sync::mpsc::channel;

use crate::opts::{APP_CONFIG_FILE_OPT, DEFAULT_MANIFEST_FILE, WATCH_DELAY_OPT};

/// Rebuild and restart the Spin application when files changes.
#[derive(Parser, Debug)]
#[clap(
    about = "Rebuild and restart the Spin application when files changes",
    allow_hyphen_values = true
)]
pub struct WatchCommand {
    /// Path to spin.toml.
    #[clap(
            name = APP_CONFIG_FILE_OPT,
            short = 'f',
            long = "file",
        )]
    pub app: Option<PathBuf>,

    /// Milliseconds to delay before rebuilding and restarting the Spin application.
    #[clap(
            name = WATCH_DELAY_OPT,
            short = 'd',
            long = "delay",
            default_value = "1000",
    )]
    pub delay: u64,

    /// Arguments to be passed through to spin up
    #[clap()]
    pub up_args: Vec<OsString>,
}

impl WatchCommand {
    pub async fn run(self) -> Result<()> {
        // Prepare manifest file and find application directory
        let manifest_file = self.app.unwrap_or_else(|| DEFAULT_MANIFEST_FILE.into());
        let app_dir = parent_dir(&manifest_file)?;
        let closure_app_dir = app_dir.clone();
        let manifest_text = tokio::fs::read_to_string(&manifest_file)
            .await
            .with_context(|| {
                format!(
                    "Cannot read manifest file from {}",
                    &manifest_file.display()
                )
            })?;
        let BuildAppInfoAnyVersion::V1(app_manifest) = toml::from_str(&manifest_text)?;

        // We use a channel to tell a tokio task to rebuild and restart the spin app
        // Send a message to kickoff the first spin build and spin up
        let (tx, mut rx) = channel(1);
        tx.send(true).await?;

        // Watch the filesystem for any changes and send a message on the channel if a relevant change occurs
        let mut debounced_watcher = new_debouncer(
            Duration::from_millis(self.delay),
            None,
            move |res: DebounceEventResult| match res {
                Ok(events) => {
                    println!("\n====================================");
                    println!("{} events grouped", events.len());
                    if events
                        .iter()
                        .map(|e| e.path.clone())
                        .filter(|p| {
                            should_watch_file(
                                app_manifest
                                    .components
                                    .first()
                                    .unwrap()
                                    .build
                                    .as_ref()
                                    .unwrap()
                                    .watch
                                    .as_ref(),
                                &closure_app_dir,
                                p,
                            )
                        })
                        .count()
                        > 0
                    {
                        tx.blocking_send(true).unwrap();
                    } else {
                        println!("====================================\n");
                    }
                }
                Err(errors) => errors.iter().for_each(|e| println!("Error {:?}", e)),
            },
        )
        .unwrap();

        // Recursively watch any files in the application directory
        debounced_watcher
            .watcher()
            .watch(&app_dir, RecursiveMode::Recursive)?;

        // Spawn a tokio task to receive messages over a channel and rerun spin build and spin up
        tokio::spawn(async move {
            // The docs for `current_exe` warn that this may be insecure because it could be executed
            // via hard-link. I think it should be fine as long as we aren't `setuid`ing this binary.
            let spin_cmd = std::env::current_exe().unwrap();
            let mut up_popen: Option<subprocess::Popen> = None;

            while let Some(b) = rx.recv().await {
                match b {
                    true => {
                        if up_popen.is_some() && up_popen.as_mut().unwrap().poll().is_none() {
                            println!("Killing up process");
                            signal::kill(
                                nix::unistd::Pid::from_raw(
                                    up_popen.as_ref().unwrap().pid().unwrap() as i32,
                                ),
                                Signal::SIGINT,
                            )
                            .unwrap();
                            println!("====================================\n");
                        }

                        let _build_status = Exec::shell(format!(
                            "{} build -f {}",
                            &spin_cmd.display(),
                            &manifest_file.display()
                        ))
                        .stdout(Redirection::None)
                        .stderr(Redirection::None)
                        .popen()
                        .unwrap()
                        .wait()
                        .unwrap();

                        up_popen = Some(
                            Exec::shell(format!(
                                "{} up -f {} {}",
                                &spin_cmd.display(),
                                &manifest_file.display(),
                                self.up_args.join(&OsString::from(" ")).to_str().unwrap()
                            ))
                            .stdout(Redirection::None)
                            .stderr(Redirection::None)
                            .popen()
                            .unwrap(),
                        );
                    }
                    false => println!("watch error"),
                }
            }
        })
        .await?;

        Ok(())
    }
}

fn should_watch_file(
    watch_list: Option<&Vec<String>>,
    app_dir: &Path,
    path_to_match: &Path,
) -> bool {
    match watch_list {
        Some(to_watch) => {
            let is_a_match = to_watch
                .iter()
                .map(|path| Pattern::new(app_dir.join(path).to_str().unwrap()).unwrap())
                .map(|pattern| pattern.matches_path(path_to_match))
                .any(|x| x);

            if is_a_match {
                println!("Matched {:?}", path_to_match);
            }
            is_a_match
        }
        None => false,
    }
}
