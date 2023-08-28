use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Parser;
use spin_loader::local::{
    config::{RawComponentManifestImpl, RawFileMount, RawModuleSource},
    parent_dir,
};
use spin_manifest::TriggerConfig;
use watchexec::{
    action::{Action, PreSpawn},
    config::{InitConfig, RuntimeConfig},
    error::RuntimeError,
    event::{Event, Priority, ProcessEnd, Tag},
    handler::SyncFnHandler,
    signal::source::MainSignal::Interrupt,
    ErrorHook, Watchexec,
};

use crate::{
    opts::{
        APP_MANIFEST_FILE_OPT, DEFAULT_MANIFEST_FILE, WATCH_CLEAR_OPT, WATCH_DEBOUNCE_OPT,
        WATCH_SKIP_BUILD_OPT,
    },
    watch_filter::{Filter, WatchPattern},
    watch_state::{Effect, Effects, State, WatchState},
};

/// Build and run the Spin application, rebuilding and restarting it when files change.
#[derive(Parser, Debug)]
#[clap(
    about = "Build and run the Spin application, rebuilding and restarting it when files change",
    allow_hyphen_values = true
)]
pub struct WatchCommand {
    /// The application to watch. This may be a manifest (spin.toml) file, or a
    /// directory containing a spin.toml file.
    /// If omitted, it defaults to "spin.toml".
    #[clap(
        name = APP_MANIFEST_FILE_OPT,
        short = 'f',
        long = "from",
        alias = "file",
        default_value = DEFAULT_MANIFEST_FILE
    )]
    pub app_source: PathBuf,

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

        let app = spin_common::paths::resolve_manifest_file_path(&self.app_source)?;

        // Prepare RuntimeConfig for Watchexec
        let app_dir = parent_dir(&app)?;
        let filter = Arc::new(Filter::new(self.generate_filter_config().await?)?);
        let watch_state = WatchState::new(self.skip_build, self.clear);
        let watch_state_clone = watch_state.clone();
        let mut runtime_config = RuntimeConfig::default();
        runtime_config.pathset([app_dir]);
        runtime_config.command_grouped(true);
        runtime_config.filterer(filter.clone());
        runtime_config.action_throttle(Duration::from_millis(self.debounce));
        runtime_config.commands(vec![watchexec::command::Command::Exec {
            prog: self.generate_command(),
            args: vec![],
        }]);
        runtime_config.on_pre_spawn(move |prespawn: PreSpawn| {
            let up_args = self.up_args.clone();
            let manifest_path = app.to_str().unwrap().to_owned();
            let watch_state = watch_state.clone();
            async move {
                // Dynamically modify the command we're running based on the watch state
                let state = watch_state.get_state();
                let spin_args = WatchCommand::generate_arguments(state, up_args, manifest_path);
                let mut cmd = prespawn.command().await.unwrap();
                cmd.args(spin_args);
                tracing::debug!("modifying command to: {cmd:?}");
                Ok::<(), Infallible>(())
            }
        });
        runtime_config.on_action(move |action: Action| {
            tracing::debug!("handling action: {action:?}");
            let filter = filter.clone();
            let watch_state = watch_state_clone.clone();
            async move {
                // Map all the events of this action to an effect
                let mut effects = Effects::new();
                for event in action.events.iter() {
                    if event.signals().any(|s| s.eq(&Interrupt)) {
                        effects.add(Effect::Exit);
                    }
                    if event
                        .tags
                        .iter()
                        .any(|t| matches!(t, Tag::ProcessCompletion(Some(ProcessEnd::Success))))
                    {
                        effects.add(Effect::ChildProcessCompleted);
                    }
                    if event.tags.iter().any(|t| {
                        matches!(
                            t,
                            Tag::ProcessCompletion(None)
                                | Tag::ProcessCompletion(Some(
                                    ProcessEnd::Exception(_)
                                        | ProcessEnd::ExitError(_)
                                        | ProcessEnd::ExitStop(_)
                                ))
                        )
                    }) {
                        effects.add(Effect::ChildProcessFailed);
                    }
                    if filter.matches_manifest_pattern(event) {
                        // TODO: Reconfigure the watcher
                        eprintln!("Application manifest has changed. If this included changes to the watch configuration, please restart Spin.");
                        effects.add(Effect::ManifestChange);
                    }
                    if filter.matches_source_pattern(event) {
                        effects.add(Effect::SourceChange);
                    }
                    if filter.matches_artifact_pattern(event) {
                        effects.add(Effect::ArtifactChange);
                    }
                }
                action.outcome(watch_state.handle(effects.reduce()));
                Ok::<(), Infallible>(())
            }
        });

        // Start watching
        let runtime = Watchexec::new(init_config, runtime_config.clone())?;
        runtime
            .send_event(Event::default(), Priority::Urgent)
            .await?;
        runtime.main().await??;
        Ok(())
    }

    fn generate_command(&self) -> String {
        // The docs for `current_exe` warn that this may be insecure because it could be executed
        // via hard-link. I think it should be fine as long as we aren't `setuid`ing this binary.
        String::from(
            std::env::current_exe()
                .unwrap()
                .to_str()
                .expect("to find exe path"),
        )
    }

    fn generate_arguments(
        state: State,
        up_args: Vec<String>,
        manifest_path: String,
    ) -> Vec<String> {
        let mut spin_args = match state {
            State::Building => vec![String::from("build")],
            State::Running => vec![String::from("up")],
            State::WaitingForSpinUpToExit => {
                // This should never happen and it's a logic error if it does.
                // Currently this tries to gracefully continue but it might mean
                // the watch state and running process are out of sync in which
                // case it might be better to panic...?
                debug_assert!(false, "spin watch: Should not have tried to start a process while in the WaitingForSpinUpToExit state");
                tracing::error!(
                    "Internal error: spin watch tried to restart while waiting for spin up to exit"
                );
                vec![String::from("build")]
            }
        };
        spin_args.append(&mut vec![String::from("-f"), manifest_path]);
        if matches!(state, State::Running) {
            spin_args.extend(up_args);
        }
        spin_args
    }

    async fn generate_filter_config(&self) -> Result<crate::watch_filter::Config> {
        let app = spin_common::paths::resolve_manifest_file_path(&self.app_source)?;
        let app_dir = parent_dir(&app)?;
        let app_manifest = spin_loader::local::raw_manifest_from_file(&app)
            .await?
            .into_v1();

        // We always want to watch the application manifest
        let manifest_pattern = WatchPattern::new(
            app.to_str()
                .with_context(|| format!("non-unicode manifest path {:?}", app))?
                .to_owned(),
            app_dir.as_path(),
        )?;

        // We want to watch the source code if we aren't skipping the build step
        let source_patterns = match !self.skip_build {
            false => vec![],
            true => app_manifest
                .components
                .iter()
                .filter_map(|c| WatchCommand::create_source_pattern(c, &app_dir))
                .flatten()
                .collect::<Result<Vec<WatchPattern>>>()?,
        };

        // We want to watch a component's source if it has no build step. If we're skipping
        // building then we'll watch all component sources.
        let component_source_patterns = app_manifest
            .components
            .iter()
            .filter_map(|c| {
                let RawModuleSource::FileReference(path) = &c.source else {
                    return None;
                };
                let path_str = path.to_str()?;
                let watch_this_source = self.skip_build || c.build.is_none();
                watch_this_source
                    .then_some(WatchPattern::new(path_str.to_owned(), app_dir.as_path()))
            })
            .collect::<Result<Vec<WatchPattern>>>()?;

        // We always want to watch component files
        let files_patterns = app_manifest
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
            })
            .map(|p| WatchPattern::new(p, app_dir.as_path()))
            .collect::<Result<Vec<WatchPattern>>>()?;

        let artifact_patterns = component_source_patterns
            .into_iter()
            .chain(files_patterns.into_iter())
            .collect();

        Ok(crate::watch_filter::Config {
            manifest_pattern,
            source_patterns,
            artifact_patterns,
            ignore_patterns: Filter::default_ignore_patterns(),
        })
    }

    fn create_source_pattern(
        c: &RawComponentManifestImpl<TriggerConfig>,
        app_dir: &Path,
    ) -> Option<Vec<Result<WatchPattern>>> {
        let build = c.build.as_ref()?;
        let Some(watch) = build.watch.clone() else {
            eprintln!(
                "You haven't configured what to watch for the component: '{}'. Learn how to configure Spin watch at https://developer.fermyon.com/common/cli-reference#watch",
                c.id
            );
            return None;
        };
        let sources = build
            .workdir
            .clone()
            .map(|workdir| {
                watch
                    .iter()
                    .filter_map(|w| workdir.join(w).to_str().map(String::from))
                    .collect()
            })
            .unwrap_or(watch);
        Some(
            sources
                .into_iter()
                .map(|s| WatchPattern::new(s, app_dir))
                .collect::<Vec<Result<WatchPattern>>>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_up_args_are_passed_through() {
        let args = WatchCommand::generate_arguments(
            State::Running,
            vec![String::from("--quiet")],
            String::from("spin.toml"),
        );
        assert_eq!(4, args.len());
        assert_eq!(String::from("up"), *args.get(0).unwrap());
        assert_eq!(String::from("-f"), *args.get(1).unwrap());
        assert_eq!(String::from("spin.toml"), *args.get(2).unwrap());
        assert_eq!(String::from("--quiet"), *args.get(3).unwrap());
    }

    #[tokio::test]
    async fn test_standard_config_proj1() {
        let app_path = "tests/watch/http-rust/spin.toml";
        let watch_command = WatchCommand {
            app_source: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: false,
            up_args: vec![],
        };
        let config = watch_command.generate_filter_config().await.unwrap();

        assert!(config.manifest_pattern.glob.ends_with("spin.toml"));

        assert_eq!(config.source_patterns.len(), 4);
        assert!(config
            .source_patterns
            .get(0)
            .unwrap()
            .glob
            .ends_with("http-rust/src/**/*.rs"));
        assert!(config
            .source_patterns
            .get(1)
            .unwrap()
            .glob
            .ends_with("http-rust/Cargo.toml"));
        assert!(config
            .source_patterns
            .get(2)
            .unwrap()
            .glob
            .ends_with("http-rust/subcomponent/**/*.go"));
        assert!(config
            .source_patterns
            .get(3)
            .unwrap()
            .glob
            .ends_with("http-rust/subcomponent/go.mod"));

        assert_eq!(config.artifact_patterns.len(), 0);

        assert_eq!(config.ignore_patterns.len(), 1);
        assert!(config
            .ignore_patterns
            .get(0)
            .unwrap()
            .glob
            .ends_with("*.swp"));
    }

    #[tokio::test]
    async fn test_skip_build_config_proj1() {
        let app_path = "tests/watch/http-rust/spin.toml";
        let watch_command = WatchCommand {
            app_source: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: true,
            up_args: vec![],
        };
        let config = watch_command.generate_filter_config().await.unwrap();

        assert_eq!(config.source_patterns.len(), 0);

        assert_eq!(config.artifact_patterns.len(), 2);
    }

    #[tokio::test]
    async fn test_standard_config_proj2() {
        let app_path = "tests/watch/static-fileserver/spin.toml";
        let watch_command = WatchCommand {
            app_source: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: false,
            up_args: vec![],
        };
        let config = watch_command.generate_filter_config().await.unwrap();

        assert_eq!(config.source_patterns.len(), 0);

        assert_eq!(config.artifact_patterns.len(), 3);
        assert!(config
            .artifact_patterns
            .get(0)
            .unwrap()
            .glob
            .ends_with("static-fileserver/spin_static_fs.wasm"));
        assert!(config
            .artifact_patterns
            .get(1)
            .unwrap()
            .glob
            .ends_with("static-fileserver/assets/**/*"));
        assert!(config
            .artifact_patterns
            .get(2)
            .unwrap()
            .glob
            .ends_with("static-fileserver/assets2/**/*"));
    }

    #[tokio::test]
    async fn test_accepts_directory() {
        let app_path = "tests/watch/static-fileserver";
        let watch_command = WatchCommand {
            app_source: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: false,
            up_args: vec![],
        };
        let config = watch_command.generate_filter_config().await.unwrap();

        assert_eq!(config.source_patterns.len(), 0);

        assert_eq!(config.artifact_patterns.len(), 3);
        assert!(config
            .artifact_patterns
            .get(0)
            .unwrap()
            .glob
            .ends_with("static-fileserver/spin_static_fs.wasm"));
        assert!(config
            .artifact_patterns
            .get(1)
            .unwrap()
            .glob
            .ends_with("static-fileserver/assets/**/*"));
        assert!(config
            .artifact_patterns
            .get(2)
            .unwrap()
            .glob
            .ends_with("static-fileserver/assets2/**/*"));
    }

    #[tokio::test]
    async fn test_skip_build_config_proj2() {
        let app_path = "tests/watch/static-fileserver/spin.toml";
        let watch_command = WatchCommand {
            app_source: app_path.into(),
            clear: false,
            debounce: 100,
            skip_build: true,
            up_args: vec![],
        };
        let config = watch_command.generate_filter_config().await.unwrap();

        assert_eq!(config.source_patterns.len(), 0);

        assert_eq!(config.artifact_patterns.len(), 3);
        assert!(config
            .artifact_patterns
            .get(0)
            .unwrap()
            .glob
            .ends_with("static-fileserver/spin_static_fs.wasm"));
        assert!(config
            .artifact_patterns
            .get(1)
            .unwrap()
            .glob
            .ends_with("static-fileserver/assets/**/*"));
        assert!(config
            .artifact_patterns
            .get(2)
            .unwrap()
            .glob
            .ends_with("static-fileserver/assets2/**/*"));
    }
}
