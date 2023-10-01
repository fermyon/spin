use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use clap::Parser;
use itertools::Itertools;
use spin_loader::local::{
    config::{RawComponentManifestImpl, RawFileMount, RawModuleSource},
    parent_dir,
};
use uuid::Uuid;
use watchexec::Watchexec;

use crate::opts::{
    APP_MANIFEST_FILE_OPT, DEFAULT_MANIFEST_FILE, WATCH_CLEAR_OPT, WATCH_DEBOUNCE_OPT,
    WATCH_SKIP_BUILD_OPT,
};

mod buildifier;
mod uppificator;

use buildifier::Buildifier;
use uppificator::{Pause, Uppificator};

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
        // Strategy:
        // * The Uppificator runs `spin up`, and watches the manifest artifacts (component.source and component.files)
        //   (and the manifest if build is not in play). When it detects a change, it restarts `spin up`.
        //   THAT'S ALL, THAT'S ALL IT DOES.
        //   * If `spin up` crashes, the Uppificator restarts it.  BUT APART FROM THAT THAT'S ALL IT DOES OKAY.
        // * The Buildifier, if in play, watches the manifest and component.build.watch collections. When it detects a
        //   change, it PAUSES the Uppificator, does the build, then unpauses the Uppificator.
        //   * It is on the Uppificator to recognise if any interesting files have changed when it unpauses.
        // * In skip_build configurations, the Buildifier is not present.
        // * In clear configurations, both the Buildifier and the Uppificator clear the screen on a change.
        //   * There is a slight twist here that the Uppificator does _not_ clear the screen if the Buildifier
        //     has just done so.  Subsequent asset changes _do_ clear the screen.

        let spin_bin = std::env::current_exe()?;
        let manifest_file = spin_common::paths::resolve_manifest_file_path(&self.app_source)?;
        let manifest_dir = parent_dir(&manifest_file)?;
        let manifest = spin_loader::local::raw_manifest_from_file(&manifest_file)
            .await?
            .into_v1();

        // Set up the event processors (but don't start them yet).
        // We create the build-related objects even if skip_build is on - the cost is insignificant,
        // and it saves having to check options all over the place.

        let (artifact_tx, artifact_rx) = tokio::sync::watch::channel(Uuid::new_v4());
        let (pause_tx, pause_rx) = tokio::sync::mpsc::channel(1);
        let (source_code_tx, source_code_rx) = tokio::sync::watch::channel(Uuid::new_v4());
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(Uuid::new_v4());

        let mut buildifier = Buildifier {
            spin_bin: spin_bin.clone(),
            manifest: manifest_file.clone(),
            clear_screen: self.clear,
            watched_changes: source_code_rx,
            uppificator_pauser: pause_tx,
        };

        let mut uppificator = Uppificator {
            spin_bin: spin_bin.clone(),
            manifest: manifest_file.clone(),
            up_args: self.up_args.clone(),
            clear_screen: self.clear,
            watched_changes: artifact_rx,
            pause_feed: pause_rx,
            stopper: stop_rx,
        };

        // Start `watchexec` tasks to monitor artifact and build files.

        // BUG: Currently the filter is not dynamic - if the asset list in the manifest changes, this will
        // trigger a reload but the old asset list remains effective. I think this was an existing issue, not
        // sure if I've exacerbated it. We can use `Watchexec::reconfigure()` to fix this but
        // then we might need a Reconfiguriser as well to monitor the manifest.

        let artifact_filterer = self
            .artifact_filterer(&manifest_file, &manifest_dir, &manifest)
            .await
            .context("Error creating artifact filterer")?;
        let artifact_watcher_handle = self
            .spawn_watchexec(&manifest_dir, artifact_filterer, artifact_tx, "reload")
            .context("Error creating artifact watcher")?;

        let build_files_watcher_handle = if self.skip_build {
            tokio::task::spawn(tokio::time::sleep(tokio::time::Duration::MAX))
        } else {
            let build_filterer = self
                .build_filterer(&manifest_file, &manifest_dir, &manifest)
                .await
                .context("Error creating build files filterer")?;
            self.spawn_watchexec(&manifest_dir, build_filterer, source_code_tx, "build")
                .context("Error creating build files watcher")?
        };

        // Start the the uppificator and buildifier to process notifications from the
        // `watchexec` tasks.

        let uppificator_handle = tokio::task::spawn(async move { uppificator.run().await });
        let buildifier_handle = if self.skip_build {
            // There's no build to complete and unpause the uppificator, so synthesise an unpause.
            let _ = buildifier.uppificator_pauser.send(Pause::Unpause).await;
            tokio::task::spawn(tokio::time::sleep(tokio::time::Duration::MAX))
        } else {
            tokio::task::spawn(async move { buildifier.run().await })
        };

        // Wait for either the user to stop the watch, or some catastrophe to dump us out.

        // When the user stops the watch, signal to the Uppificator to stop `spin up` and
        // break out its run loop. This will cause the `uppificator_handle` future to
        // complete, which will cause the select to return.
        _ = ctrlc::set_handler(move || {
            _ = stop_tx.send(Uuid::new_v4());
        });

        // As noted above, the most likely future to complete is the uppificator on a Ctrl+C.
        // But if any future completes, the watch can no longer continue. So we use a select
        // so that any completion causes us to fall through to program exit. (And program
        // exit kills the remaining futures.)
        _ = futures::future::select_all(vec![
            uppificator_handle,
            buildifier_handle,
            artifact_watcher_handle,
            build_files_watcher_handle,
        ])
        .await;

        Ok(())
    }

    // `watchexec` is normally used to restart a process on a change. In our case, we
    // manage process stops and starts manually via the Uppificator and Buildifier,
    // but watchexec still has valuable change detection functionality.  (Specifically,
    // it has the filtering feature, which didn't seem to come out of the box with things
    // like `notify`.) Therefore, we use `watchexec`, but do not set a command to run,
    // and always set the action outcome to DoNothing. Instead, our action handler manually
    // sends messages to let the two process managers make their own decisions.
    fn spawn_watchexec(
        &self,
        manifest_dir: &Path,
        filterer: impl watchexec::filter::Filterer + 'static,
        notifier: tokio::sync::watch::Sender<Uuid>,
        impact_description: &'static str,
    ) -> anyhow::Result<tokio::task::JoinHandle<()>> {
        let mut despurifier = despurifier::Despurifier::new();
        let mut rt = watchexec::config::RuntimeConfig::default();
        rt.pathset([manifest_dir]);
        rt.filterer(Arc::new(filterer));
        rt.action_throttle(Duration::from_millis(self.debounce));
        rt.on_action(move |action: watchexec::action::Action| {
            if despurifier.all_spurious(&action) {
                tracing::debug!("spin watch ignored spurious changes: {}", paths_of(&action));
            } else {
                tracing::debug!(
                    "spin watch detected changes requiring {impact_description}: {}",
                    paths_of(&action)
                );
                _ = notifier.send(Uuid::new_v4());
            }
            action.outcome(watchexec::action::Outcome::DoNothing);
            async { Ok::<(), Infallible>(()) }
        });
        let watcher = Watchexec::new(watchexec::config::InitConfig::default(), rt)?;
        let watcher_join_handle = tokio::task::spawn(async move {
            _ = watcher.main().await;
        });
        Ok(watcher_join_handle)
    }

    async fn build_filterer<T>(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        manifest: &spin_loader::local::config::RawAppManifestImpl<T>,
    ) -> anyhow::Result<watchexec_filterer_globset::GlobsetFilterer> {
        let manifest_glob = vec![stringize_path(manifest_file)?];
        let src_globs = manifest
            .components
            .iter()
            .filter_map(|c| Self::create_source_globs(c))
            .flatten();

        let build_globs = manifest_glob
            .into_iter()
            .chain(src_globs)
            .map(|s| (s, None))
            .collect::<Vec<_>>();

        let filterer = watchexec_filterer_globset::GlobsetFilterer::new(
            manifest_dir,
            build_globs,
            standard_ignores(),
            [],
            [],
        )
        .await?;

        Ok(filterer)
    }

    async fn artifact_filterer<T>(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        manifest: &spin_loader::local::config::RawAppManifestImpl<T>,
    ) -> anyhow::Result<watchexec_filterer_globset::GlobsetFilterer> {
        let manifest_glob = if self.skip_build {
            vec![stringize_path(manifest_file)?]
        } else {
            vec![] // In this case, manifest changes trigger a rebuild, which will poke the uppificator anyway
        };
        let wasm_globs = manifest.components.iter().filter_map(|c| {
            let RawModuleSource::FileReference(path) = &c.source else {
                return None;
            };
            path.to_str().map(String::from)
        });
        let asset_globs = manifest
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

        let artifact_globs = manifest_glob
            .into_iter()
            .chain(wasm_globs)
            .chain(asset_globs)
            .map(|s| (s, None))
            .collect::<Vec<_>>();

        let filterer = watchexec_filterer_globset::GlobsetFilterer::new(
            manifest_dir,
            artifact_globs,
            standard_ignores(),
            [],
            [],
        )
        .await?;

        Ok(filterer)
    }

    fn create_source_globs<T>(c: &RawComponentManifestImpl<T>) -> Option<Vec<String>> {
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
        Some(sources)
    }
}

fn standard_ignores() -> Vec<(String, Option<PathBuf>)> {
    [
        "**/*.swp", // Vim creates swap files during editing
    ]
    .into_iter()
    .map(|pat| (pat.to_owned(), None))
    .collect()
}

fn paths_of(action: &watchexec::action::Action) -> String {
    action
        .events
        .iter()
        .filter_map(path_of_event)
        .map(|p| format!("{}", p.display()))
        .unique()
        .collect_vec()
        .join(", ")
}

fn path_of_event(event: &watchexec::event::Event) -> Option<&Path> {
    event.tags.iter().filter_map(path_of_tag).next()
}

fn path_of_tag(tag: &watchexec::event::Tag) -> Option<&Path> {
    match tag {
        watchexec::event::Tag::Path { path, .. } => Some(path),
        _ => None,
    }
}

fn stringize_path(path: &Path) -> anyhow::Result<String> {
    match path.to_str() {
        Some(s) => Ok(s.to_owned()),
        None => bail!("Can't represent path {} as string", path.display()),
    }
}

#[cfg(target_os = "macos")]
mod despurifier {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::SystemTime;
    use watchexec::event::{filekind::FileEventKind, Tag};

    pub struct Despurifier {
        process_start_time: SystemTime,
        files_modified_at: Mutex<HashMap<String, SystemTime>>,
    }

    impl Despurifier {
        pub fn new() -> Self {
            Self {
                process_start_time: SystemTime::now(),
                files_modified_at: Mutex::new(HashMap::new()),
            }
        }

        pub fn all_spurious(&mut self, action: &watchexec::action::Action) -> bool {
            action.events.iter().all(|e| self.all_spurious_evt(e))
        }

        // This is necessary to check due to a bug on macOS emitting modify events on copies
        // https://github.com/rust-lang/rust/issues/107130
        fn all_spurious_evt(&mut self, event: &watchexec::event::Event) -> bool {
            // Deletions are never spurious
            if event
                .tags
                .iter()
                .any(|tag| matches!(tag, Tag::FileEventKind(FileEventKind::Remove(_))))
            {
                return false;
            }

            for (path, _) in event.paths() {
                let Ok(metadata) = std::fs::metadata(path) else {
                    continue;
                };
                let Ok(path_time) = metadata.modified() else {
                    continue;
                };
                let mut modified_map = self.files_modified_at.lock().unwrap();
                let path_key = match path.to_str() {
                    Some(s) => s.to_owned(),
                    None => {
                        tracing::warn!("can't check non-unicode path: {path:?}");
                        continue;
                    }
                };
                let base_time = modified_map
                    .get(&path_key)
                    .unwrap_or(&self.process_start_time);
                if &path_time > base_time {
                    modified_map.insert(path_key, path_time);
                    return false;
                }
            }

            true
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod despurifier {
    pub struct Despurifier;

    impl Despurifier {
        pub fn new() -> Self {
            Self
        }

        pub fn all_spurious(&mut self, _action: &watchexec::action::Action) -> bool {
            false
        }
    }
}
