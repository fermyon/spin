use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};

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

#[derive(Debug)]
pub struct WatchFilter {
    path_patterns: Vec<Pattern>,
    ignore_patterns: Vec<Pattern>,
    process_start_time: SystemTime,
    files_modified_at: Arc<Mutex<HashMap<String, SystemTime>>>,
}

impl WatchFilter {
    pub fn new(
        app_dir: PathBuf,
        path_patterns: Vec<String>,
        ignore_patterns: Vec<String>,
    ) -> Result<Self> {
        tracing::info!("watching relative to app dir: {app_dir:?}");
        tracing::info!("watching path patterns: {path_patterns:?}");
        tracing::info!("ignoring path patterns: {ignore_patterns:?}");

        Ok(WatchFilter {
            path_patterns: path_patterns
                .iter()
                .map(|path| WatchFilter::app_dir_pattern(&app_dir, path))
                .collect::<Result<_>>()?,
            ignore_patterns: ignore_patterns
                .iter()
                .map(|path| WatchFilter::app_dir_pattern(&app_dir, path))
                .collect::<Result<_>>()?,
            process_start_time: SystemTime::now(),
            files_modified_at: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// A default set of ignore patterns that can be used by the consumer of WatchFilter.
    pub fn default_ignore_patterns() -> Vec<String> {
        vec![String::from("*.swp")]
    }

    fn app_dir_pattern(app_dir: &Path, path: &str) -> anyhow::Result<Pattern> {
        let pat = app_dir.join(path);
        let pat_str = pat
            .to_str()
            .with_context(|| format!("non-unicode pattern {path:?}"))?;
        Pattern::new(pat_str).with_context(|| format!("invalid glob pattern {path:?}"))
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

    fn matches_one_of_ignore_patterns(&self, event: &Event) -> bool {
        event.paths().any(|(path, _)| {
            self.ignore_patterns
                .iter()
                .any(|pattern| pattern.matches_path(path))
        })
    }

    fn matches_one_of_path_patterns(&self, event: &Event) -> bool {
        event.paths().any(|(path, _)| {
            self.path_patterns
                .iter()
                .any(|pattern| pattern.matches_path(path))
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

impl Filterer for WatchFilter {
    fn check_event(&self, event: &Event, _: Priority) -> std::result::Result<bool, RuntimeError> {
        // All interrupt signals are allowed through
        for signal in event.signals() {
            if let Interrupt = signal {
                tracing::trace!("passing event b/c of interrupt signal: {event:?}");
                return Ok(true);
            }
        }

        // Fail if the event kind wasn't creation, modification, or deletion
        if !self.has_valid_event_kind(event) {
            tracing::trace!("failing event b/c of event kind: {event:?}");
            return Ok(false);
        }

        // Fail if a path matches the ignored patterns
        if self.matches_one_of_ignore_patterns(event) {
            tracing::trace!("failing event b/c it matches ignore pattern: {event:?}");
            return Ok(false);
        }

        // Fail if a path doesn't match one of the given path patterns
        if !self.matches_one_of_path_patterns(event) {
            tracing::trace!("failing event b/c it doesn't match path pattern: {event:?}");
            return Ok(false);
        }

        // Fail if a path metadata doesn't actually show it is has been modified
        if cfg!(target_os = "macos") {
            match self.path_has_been_actually_modified(event) {
                Ok(true) => {}
                Ok(false) => {
                    tracing::trace!("failing event b/c it wasn't actually modified: {event:?}");
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
        tracing::trace!("passing event: {event:?}");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

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

    fn make_filter(root: PathBuf, path_patterns: Vec<String>) -> WatchFilter {
        WatchFilter::new(root, path_patterns, WatchFilter::default_ignore_patterns()).unwrap()
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
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]);

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
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]);

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

        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]);

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
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]);

        fs::remove_file(file_path.clone()).unwrap();

        let event = make_event(file_path, FileEventKind::Remove(RemoveKind::File));

        let result = filter.check_event(&event, Priority::Normal);
        assert!(matches!(result, Ok(true)));
    }

    #[test]
    fn test_modify_metadata_watched_file() {
        let (root, file_path, file) = make_directory_and_file("rs");
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]);

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
        let filter = make_filter(root.path().to_path_buf(), vec![String::from("**/*.rs")]);

        let event = make_event(
            file_path,
            FileEventKind::Modify(ModifyKind::Data(DataChange::Content)),
        );

        let result = filter.check_event(&event, Priority::Normal);
        assert!(matches!(result, Ok(false)));
    }
}
