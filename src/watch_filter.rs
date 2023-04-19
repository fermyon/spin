use std::{collections::HashMap, fmt, path::Path, sync::Mutex, time::SystemTime};

use anyhow::{Context, Result};
use glob::Pattern;
use watchexec::{
    error::RuntimeError,
    event::{
        filekind::{FileEventKind, ModifyKind},
        Event, Priority, Tag,
    },
    filter::Filterer,
    signal::source::MainSignal::Interrupt,
};

/// Filters Watchexec events.
///
/// Also enables checking whether an application manifest, source, or artifact was modified.
#[derive(Debug)]
pub struct Filter {
    config: Config,
    process_start_time: SystemTime,
    files_modified_at: Mutex<HashMap<String, SystemTime>>,
}

impl Filter {
    pub fn new(config: Config) -> Result<Self> {
        tracing::debug!("watching: {:?}", config);

        Ok(Filter {
            config,
            process_start_time: SystemTime::now(),
            files_modified_at: Mutex::new(HashMap::new()),
        })
    }

    /// A default set of ignore patterns that can be used by the consumer of Filter.
    pub fn default_ignore_patterns() -> Vec<WatchPattern> {
        vec!["*.swp"]
            .iter()
            .map(|i| WatchPattern {
                glob: i.to_string(),
                pattern: Pattern::new(i).unwrap(),
            })
            .collect()
    }

    /// Determine if an event has any paths matching the manifest pattern
    pub fn matches_manifest_pattern(&self, event: &Event) -> bool {
        event
            .paths()
            .any(|(path, _)| self.config.manifest_pattern.pattern.matches_path(path))
    }

    /// Determine if an event has any paths matching the source patterns
    pub fn matches_source_pattern(&self, event: &Event) -> bool {
        event.paths().any(|(path, _)| {
            self.config
                .source_patterns
                .iter()
                .any(|wp| wp.pattern.matches_path(path))
        })
    }

    /// Determine if an event has any paths matching the artifact patterns
    pub fn matches_artifact_pattern(&self, event: &Event) -> bool {
        event.paths().any(|(path, _)| {
            self.config
                .artifact_patterns
                .iter()
                .any(|wp| wp.pattern.matches_path(path))
        })
    }

    fn has_valid_event_kind(&self, event: &Event) -> bool {
        event.tags.iter().any(|tag| {
            matches!(
                tag,
                Tag::FileEventKind(
                    FileEventKind::Modify(
                        ModifyKind::Data(_) | ModifyKind::Name(_) | ModifyKind::Any
                    ) | FileEventKind::Create(_)
                        | FileEventKind::Remove(_)
                )
            )
        })
    }

    /// Used by `check_event` to see if the event matches any patterns that we should watch.
    fn matches_one_of_watched_patterns(&self, event: &Event) -> bool {
        self.matches_manifest_pattern(event)
            || self.matches_source_pattern(event)
            || self.matches_artifact_pattern(event)
    }

    /// Used by `check_event` to see if the event matches any ignore patterns that we should watch.
    fn matches_one_of_ignore_patterns(&self, event: &Event) -> bool {
        event.paths().any(|(path, _)| {
            self.config
                .ignore_patterns
                .iter()
                .any(|wp| wp.pattern.matches_path(path))
        })
    }

    /// This is necessary to check due to a bug on macOS emitting modify events on copies
    /// https://github.com/rust-lang/rust/issues/107130
    fn path_has_been_actually_modified(&self, event: &Event) -> Result<bool> {
        // No need to check if a deleted path has been modified because it is gone
        if event
            .tags
            .iter()
            .any(|tag| matches!(tag, Tag::FileEventKind(FileEventKind::Remove(_))))
        {
            return Ok(true);
        }

        for (path, _) in event.paths() {
            let metadata = std::fs::metadata(path)?;
            let path_time = metadata.modified()?;
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
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Filterer for Filter {
    fn check_event(&self, event: &Event, _: Priority) -> std::result::Result<bool, RuntimeError> {
        // All interrupt signals are allowed through
        for signal in event.signals() {
            if let Interrupt = signal {
                tracing::debug!("passing event (interrupt signal): {event:?}");
                return Ok(true);
            }
        }

        // All process completion events are allowed through
        if event
            .tags
            .iter()
            .any(|t| matches!(t, Tag::ProcessCompletion(_)))
        {
            tracing::debug!("passing event (process completion): {event:?}");
            return Ok(true);
        }

        // Fail if the event kind wasn't creation, modification, or deletion
        if !self.has_valid_event_kind(event) {
            tracing::trace!("failing event (irrelevant event kind): {event:?}");
            return Ok(false);
        }

        // Fail if a path matches the ignored patterns
        if self.matches_one_of_ignore_patterns(event) {
            tracing::trace!("failing event (matches ignore pattern): {event:?}");
            return Ok(false);
        }

        // Fail if a path doesn't match one of the given path patterns
        if !self.matches_one_of_watched_patterns(event) {
            tracing::trace!(
                "failing event (doesn't match source/artifact/manifest pattern): {event:?}"
            );
            return Ok(false);
        }

        // Fail if a path metadata doesn't actually show it is has been modified
        if cfg!(target_os = "macos") {
            match self.path_has_been_actually_modified(event) {
                Ok(true) => {}
                Ok(false) => {
                    tracing::trace!("failing event (wasn't actually modified): {event:?}");
                    return Ok(false);
                }
                Err(err) => {
                    tracing::warn!(
                        "failed to check if path(s) for event ({event:?}) has been modified: {err}",
                    );
                    return Ok(false);
                }
            }
        }

        // By process of elimination the event is valid
        tracing::debug!("passing event: {event:?}");
        Ok(true)
    }
}

/// Configuration for the watch filter.
#[derive(Debug)]
pub struct Config {
    pub manifest_pattern: WatchPattern,
    pub source_patterns: Vec<WatchPattern>,
    pub artifact_patterns: Vec<WatchPattern>,
    pub ignore_patterns: Vec<WatchPattern>,
}

/// Describes a glob file pattern that should be watched.
pub struct WatchPattern {
    /// String version of the absolute glob pattern.
    pub glob: String,
    /// Absolute glob pattern.
    pub pattern: Pattern,
}

impl WatchPattern {
    pub fn new(glob: String, app_dir: &Path) -> Result<Self> {
        let new_glob = app_dir.join(glob.clone());
        let new_glob_str = new_glob
            .to_str()
            .with_context(|| format!("non-unicode pattern {glob:?}"))?;
        Ok(WatchPattern {
            glob: new_glob_str.to_owned(),
            pattern: Pattern::new(new_glob_str)
                .with_context(|| format!("invalid glob pattern {glob:?}"))?,
        })
    }
}

impl fmt::Debug for WatchPattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WatchPattern")
            .field("glob", &self.glob)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    use watchexec::event::filekind::{CreateKind, DataChange, MetadataKind, RemoveKind};
    use watchexec::event::{FileType, Source};

    use super::*;

    fn make_directory_and_file(ext: &str) -> (tempfile::TempDir, PathBuf, fs::File) {
        let root = tempfile::tempdir().unwrap();
        let mut file_path = root.path().join("a");
        file_path.set_extension(ext);
        let file = fs::File::create(file_path.clone()).unwrap();
        (root, file_path, file)
    }

    fn make_filter(root: PathBuf, source_patterns: Vec<String>) -> Result<Filter> {
        // Not particularly relevant whether globs are sources or artifacts
        Filter::new(Config {
            manifest_pattern: WatchPattern::new("spin.toml".to_owned(), root.as_path())?,
            source_patterns: source_patterns
                .iter()
                .map(|p| WatchPattern::new(p.to_owned(), root.as_path()))
                .collect::<Result<Vec<WatchPattern>>>()?,
            artifact_patterns: vec![],
            ignore_patterns: Filter::default_ignore_patterns(),
        })
    }

    fn make_event(file_path: PathBuf, file_event_kind: FileEventKind) -> Event {
        Event {
            tags: vec![
                Tag::Source(Source::Filesystem),
                Tag::FileEventKind(file_event_kind),
                Tag::Path {
                    path: file_path,
                    file_type: Some(FileType::File),
                },
            ],
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_modify_watched_file() {
        let (root, file_path, mut file) = make_directory_and_file("rs");
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]).unwrap();

        file.write_all(b"hello world!").unwrap();

        let event = make_event(
            file_path,
            FileEventKind::Modify(ModifyKind::Data(DataChange::Content)),
        );

        let result = filter.check_event(&event, Priority::Normal);
        assert!(matches!(result, Ok(true)));
    }

    #[test]
    fn test_modify_ignored_file() {
        let (root, file_path, mut file) = make_directory_and_file("swp");
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]).unwrap();

        file.write_all(b"hello world!").unwrap();

        let event = make_event(
            file_path,
            FileEventKind::Modify(ModifyKind::Data(DataChange::Content)),
        );

        let result = filter.check_event(&event, Priority::Normal);
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn test_create_watched_file() {
        let root = tempfile::tempdir().unwrap();

        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]).unwrap();

        let mut file_path = root.path().join("a");
        file_path.set_extension("rs");
        fs::File::create(file_path.clone()).unwrap();

        let event = make_event(file_path, FileEventKind::Create(CreateKind::File));

        let result = filter.check_event(&event, Priority::Normal);
        assert!(matches!(result, Ok(true)));
    }

    #[test]
    fn test_remove_watched_file() {
        let (root, file_path, _file) = make_directory_and_file("rs");
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]).unwrap();

        fs::remove_file(file_path.clone()).unwrap();

        let event = make_event(file_path, FileEventKind::Remove(RemoveKind::File));

        let result = filter.check_event(&event, Priority::Normal);
        assert!(matches!(result, Ok(true)));
    }

    #[test]
    fn test_modify_metadata_watched_file() {
        let (root, file_path, file) = make_directory_and_file("rs");
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]).unwrap();

        let mut perms = file.metadata().unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(file_path.clone(), perms).unwrap();

        let event = make_event(
            file_path,
            FileEventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions)),
        );

        let result = filter.check_event(&event, Priority::Normal);
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_erroneously_modify_watched_file() {
        let (root, file_path, _file) = make_directory_and_file("rs");
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]).unwrap();

        let event = make_event(
            file_path,
            FileEventKind::Modify(ModifyKind::Data(DataChange::Content)),
        );

        let result = filter.check_event(&event, Priority::Normal);
        assert!(matches!(result, Ok(false)));
    }
}
