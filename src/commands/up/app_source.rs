use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum AppSource {
    None,
    File(PathBuf),
    OciRegistry(String),
    Unresolvable(String),
}

impl AppSource {
    pub fn infer_source(source: &str) -> Self {
        let path = PathBuf::from(source);
        if path.exists() {
            Self::infer_file_source(path)
        } else if spin_oci::is_probably_oci_reference(source) {
            Self::OciRegistry(source.to_owned())
        } else {
            Self::Unresolvable(format!("File or directory '{source}' not found. If you meant to load from a registry, use the `--from-registry` option."))
        }
    }

    pub fn infer_file_source(path: impl Into<PathBuf>) -> Self {
        match spin_common::paths::resolve_manifest_file_path(path.into()) {
            Ok(file) => Self::File(file),
            Err(e) => Self::Unresolvable(e.to_string()),
        }
    }

    pub fn unresolvable(message: impl Into<String>) -> Self {
        Self::Unresolvable(message.into())
    }

    pub fn local_app_dir(&self) -> Option<&Path> {
        match self {
            Self::File(path) => path.parent().or_else(|| {
                tracing::warn!("Error finding local app dir from source {path:?}");
                None
            }),
            _ => None,
        }
    }

    pub async fn build(&self) -> anyhow::Result<()> {
        match self {
            Self::File(path) => spin_build::build(path, &[]).await,
            _ => Ok(()),
        }
    }
}

impl std::fmt::Display for AppSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "<no source>"),
            Self::File(path) => write!(f, "local app {path:?}"),
            Self::OciRegistry(reference) => write!(f, "remote app {reference:?}"),
            Self::Unresolvable(s) => write!(f, "unknown app source: {s:?}"),
        }
    }
}
