use std::{convert::Infallible, path::PathBuf, sync::Arc, time::Duration};

use anyhow::Result;
use clap::Parser;
use spin_loader::local::{
    config::{RawFileMount, RawModuleSource},
    parent_dir,
};
use watchexec::{
    action::{Action, Outcome},
    config::{InitConfig, RuntimeConfig},
    error::RuntimeError,
    event::{Event, Priority},
    handler::SyncFnHandler,
    signal::source::MainSignal::Interrupt,
    ErrorHook, Watchexec,
};

use crate::{
    opts::{
        APP_MANIFEST_FILE_OPT, DEFAULT_MANIFEST_FILE, WATCH_CLEAR_OPT, WATCH_DEBOUNCE_OPT,
        WATCH_SKIP_BUILD_OPT,
    },
    watch_filter::WatchFilter,
};

/// Build and run the Spin application, rebuilding and restarting it when files change.
#[derive(Parser, Debug)]
#[clap(
    about = "Build and run the Spin application, rebuilding and restarting it when files change",
    allow_hyphen_values = true
)]
pub struct WatchCommand {
    /// Path to spin.toml.
    #[clap(
            name = APP_MANIFEST_FILE_OPT,
            short = 'f',
            long = "file",
            default_value = DEFAULT_MANIFEST_FILE,
        )]
    pub app: PathBuf,

    /// Clear the screen before each run.
    #[clap(
            name = WATCH_CLEAR_OPT,
            short = 'c',
            long = "clear",
    )]
    pub clear: bool,

    /// Set the timeout between detected change and re-execution, in milliseconds.
    #[clap(
        name = WATCH_DEBOUNCE_OPT,
        short = 'd',
        long = "debounce",
        default_value = "100"
    )]
    pub debounce: u64,

    /// Only run the Spin application, restarting it when build artifacts change.
    #[clap(name = WATCH_SKIP_BUILD_OPT, long = "skip-build")]
    pub skip_build: bool,

    /// Arguments to be passed through to spin up.
    #[clap()]
    pub up_args: Vec<String>,
}

impl WatchCommand {
    pub async fn run(self) -> Result<()> {
        // Prepare InitConfig for Watchexec
        let mut init_config = InitConfig::default();
        init_config.on_error(SyncFnHandler::from(
            |err: ErrorHook| -> std::result::Result<(), Infallible> {
                // This is a spurious error that we don't want to log
                if let RuntimeError::IoError {
                    about: "waiting on process group",
                    ..
                } = err.error
                {
                    return Ok(());
                }

                tracing::error!("{}", err.error);
                Ok(())
            },
        ));

        // Prepare RuntimeConfig for Watchexec
        let mut runtime_config = RuntimeConfig::default();
        let (spin_cmd, spin_args) = self.generate_command();
        runtime_config.commands(vec![watchexec::command::Command::Exec {
            prog: spin_cmd,
            args: spin_args,
        }]);
        let app_dir = parent_dir(&self.app)?;
        runtime_config.pathset([app_dir.clone()]);
        let filter = WatchFilter::new(
            app_dir.clone(),
            self.generate_path_patterns().await?,
            WatchFilter::default_ignore_patterns(),
        )?;
        runtime_config.command_grouped(true);
        runtime_config.filterer(Arc::new(filter));
        runtime_config.action_throttle(Duration::from_millis(self.debounce));
        runtime_config.on_action(move |action: Action| {
            let app_manifest_path = self.app.clone();
            async move {
                for event in action.events.iter() {
                    // Exit if interrupt signal sent
                    if event.signals().any(|s| s.eq(&Interrupt)) {
                        action.outcome(Outcome::both(Outcome::Stop, Outcome::Exit));
                        return Ok::<(), Infallible>(());
                    }

                    if event.paths().any(|(p, _)| p.ends_with(app_manifest_path.clone())) {
                        // TODO: Reconfigure the watcher
                        eprintln!("Application manifest has changed. If this included changes to the watch configuration, please restart Spin.");
                    }
                }

                action.outcome(Outcome::if_running(
                    Outcome::both(
                        Outcome::both(
                            Outcome::Stop,
                            match self.clear {
                                true => Outcome::Clear,
                                false => Outcome::DoNothing,
                            },
                        ),
                        Outcome::Start,
                    ),
                    Outcome::Start,
                ));

            Ok::<(), Infallible>(())
        }});

        // Start watching
        let runtime = Watchexec::new(init_config, runtime_config.clone())?;
        runtime
            .send_event(Event::default(), Priority::Urgent)
            .await?;
        runtime.main().await??;
        Ok(())
    }

    fn generate_command(&self) -> (String, Vec<String>) {
        // The docs for `current_exe` warn that this may be insecure because it could be executed
        // via hard-link. I think it should be fine as long as we aren't `setuid`ing this binary.
        let spin_cmd = String::from(
            std::env::current_exe()
                .unwrap()
                .to_str()
                .expect("to find exe path"),
        );
        let mut spin_args = match self.skip_build {
            false => vec![String::from("build"), String::from("--up")],
            true => vec![String::from("up")],
        };
        spin_args.append(&mut vec![
            String::from("-f"),
            self.app.clone().to_str().unwrap().to_owned(),
        ]);
        spin_args.append(
            self.up_args
                .clone()
                .into_iter()
                .collect::<Vec<String>>()
                .as_mut(),
        );
        tracing::info!(
            "proceeding with command: {} {}",
            spin_cmd,
            spin_args.join(" ")
        );
        (spin_cmd, spin_args)
    }

    async fn generate_path_patterns(&self) -> Result<Vec<String>> {
        // TODO: Spend some time cleaning up this function
        let app_manifest = spin_loader::local::raw_manifest_from_file(&self.app)
            .await?
            .into_v1();

        let watch_patterns = app_manifest
            .components
            .iter()
            .filter_map(|c| {
                if let Some(build) = c.build.as_ref() {
                    if build.watch.is_none() {
                        // TODO: Make sure that documentation link is pointing specifically to spin watch docs
                        eprintln!("You haven't configured what to watch for the component: '{}'. Learn how to configure Spin watch at https://developer.fermyon.com", c.id);
                    }
                    build.watch.clone()
                } else {
                    // No build config for this component so lets watch the source instead
                    if let RawModuleSource::FileReference(path) = &c.source {
                        if let Some(string_path) = path.to_str() {
                            return Some(vec![String::from(string_path)]);
                        }
                    }
                    None
                }
            })
            .flatten();
        let component_source_patterns = app_manifest
            .components
            .iter()
            .filter_map(|c| {
                if let RawModuleSource::FileReference(path) = &c.source {
                    return path.to_str();
                }
                None
            })
            .map(String::from);
        let component_file_patterns = app_manifest
            .components
            .iter()
            .filter_map(|c| c.wasm.files.as_ref())
            .flatten()
            .filter_map(|raw_file_mount| match raw_file_mount {
                RawFileMount::Placement(raw_directory_placement) => raw_directory_placement
                    .source
                    .join("**/*")
                    .to_str()
                    .map(String::from),
                RawFileMount::Pattern(pattern) => Some(pattern.to_string()),
            });

        let mut path_patterns: Vec<String> = match self.skip_build {
            false => watch_patterns.chain(component_file_patterns).collect(),
            true => component_source_patterns
                .chain(component_file_patterns)
                .collect(),
        };
        // Always watch spin.toml
        path_patterns.push(String::from(
            self.app.to_str().expect("conversion to str not to fail"),
        ));

        Ok(path_patterns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_build_arguments() {
        let app_path = "a/path/to/my/app/spin.toml";
        let watch_command = WatchCommand {
            app: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: false,
            up_args: vec!["--quiet".into()],
        };
        let (_, args) = watch_command.generate_command();
        assert_eq!(args, vec!["build", "--up", "-f", app_path, "--quiet"]);
    }

    #[test]
    fn test_skip_build_arguments() {
        let app_path = "a/path/to/my/app/spin.toml";
        let watch_command = WatchCommand {
            app: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: true,
            up_args: vec![],
        };
        let (_, args) = watch_command.generate_command();
        assert_eq!(args, vec!["up", "-f", app_path]);
    }

    #[tokio::test]
    async fn test_standard_path_patterns() {
        let app_path = "tests/watch/http-rust/spin.toml";
        let watch_command = WatchCommand {
            app: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: false,
            up_args: vec![],
        };
        let path_patterns = watch_command.generate_path_patterns().await.unwrap();
        assert_eq!(path_patterns.len(), 3);
        assert_eq!(path_patterns.get(0), Some(&String::from("src/**/*.rs")));
        assert_eq!(path_patterns.get(1), Some(&String::from("Cargo.toml")));
        assert_eq!(
            path_patterns.get(2),
            Some(&String::from("tests/watch/http-rust/spin.toml"))
        );
    }

    #[tokio::test]
    async fn test_standard_path_patterns_skipping_build() {
        let app_path = "tests/watch/http-rust/spin.toml";
        let watch_command = WatchCommand {
            app: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: true,
            up_args: vec![],
        };
        let path_patterns = watch_command.generate_path_patterns().await.unwrap();
        assert_eq!(path_patterns.len(), 2);
        assert_eq!(
            path_patterns.get(0),
            Some(&String::from("target/wasm32-wasi/release/http_rust.wasm"))
        );
        assert_eq!(
            path_patterns.get(1),
            Some(&String::from("tests/watch/http-rust/spin.toml"))
        );
    }

    #[tokio::test]
    async fn test_asset_path_patterns() {
        let app_path = "tests/watch/static-fileserver/spin.toml";
        let watch_command = WatchCommand {
            app: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: false,
            up_args: vec!["--quiet".into()],
        };
        let path_patterns = watch_command.generate_path_patterns().await.unwrap();
        assert_eq!(path_patterns.len(), 4);
        assert_eq!(
            path_patterns.get(0),
            Some(&String::from("spin_static_fs.wasm"))
        );
        assert_eq!(path_patterns.get(1), Some(&String::from("assets/**/*")));
        assert_eq!(path_patterns.get(2), Some(&String::from("assets2/**/*")));
        assert_eq!(
            path_patterns.get(3),
            Some(&String::from("tests/watch/static-fileserver/spin.toml"))
        );
    }

    #[tokio::test]
    async fn test_asset_path_patterns_skipping_build() {
        let app_path = "tests/watch/static-fileserver/spin.toml";
        let watch_command = WatchCommand {
            app: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: true,
            up_args: vec!["--quiet".into()],
        };
        let path_patterns = watch_command.generate_path_patterns().await.unwrap();
        assert_eq!(path_patterns.len(), 4);
        assert_eq!(
            path_patterns.get(0),
            Some(&String::from("spin_static_fs.wasm"))
        );
        assert_eq!(path_patterns.get(1), Some(&String::from("assets/**/*")));
        assert_eq!(path_patterns.get(2), Some(&String::from("assets2/**/*")));
        assert_eq!(
            path_patterns.get(3),
            Some(&String::from("tests/watch/static-fileserver/spin.toml"))
        );
    }
}
