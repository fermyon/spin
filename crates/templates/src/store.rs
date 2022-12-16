use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use sha2::{Digest, Sha256};

use crate::directory::subdirectories;

pub(crate) struct TemplateStore {
    root: PathBuf,
}

lazy_static::lazy_static! {
    static ref UNSAFE_CHARACTERS: regex::Regex = regex::Regex::new("[^-_a-zA-Z0-9]").expect("Invalid identifier regex");
}

impl TemplateStore {
    pub(crate) fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_owned(),
        }
    }

    pub(crate) fn try_default() -> anyhow::Result<Self> {
        let data_dir = dirs::data_local_dir()
            .or_else(|| dirs::home_dir().map(|p| p.join(".spin")))
            .ok_or_else(|| anyhow!("Unable to get local data directory or home directory"))?;
        let templates_dir = data_dir.join("spin").join("templates");
        Ok(Self::new(templates_dir))
    }

    pub(crate) fn get_directory(&self, id: impl AsRef<str>) -> PathBuf {
        self.root.join(Self::relative_dir(id.as_ref()))
    }

    pub(crate) fn get_layout(&self, id: impl AsRef<str>) -> Option<TemplateLayout> {
        let template_dir = self.get_directory(id);
        if template_dir.exists() {
            Some(TemplateLayout::new(&template_dir))
        } else {
            None
        }
    }

    pub(crate) async fn list_layouts(&self) -> anyhow::Result<Vec<TemplateLayout>> {
        if !self.root.exists() {
            return Ok(vec![]);
        }

        let template_dirs = subdirectories(&self.root).with_context(|| {
            format!(
                "Failed to read template directories from {}",
                self.root.display()
            )
        })?;

        Ok(template_dirs.iter().map(TemplateLayout::new).collect())
    }

    fn relative_dir(id: &str) -> impl AsRef<Path> {
        // Using the SHA could generate quite long directory names, which could be a problem on Windows
        // if the template filenames are also long. Longer term, consider an alternative approach where
        // we use an index or something for disambiguation, and/or disambiguating only if a clash is
        // detected, etc.
        let id_sha256 = format!("{:x}", Sha256::digest(id));
        format!("{}_{}", UNSAFE_CHARACTERS.replace_all(id, "_"), id_sha256)
    }
}

pub(crate) struct TemplateLayout {
    template_dir: PathBuf,
}

const METADATA_DIR_NAME: &str = "metadata";
const FILTERS_DIR_NAME: &str = "filters";
const CONTENT_DIR_NAME: &str = "content";
const SNIPPETS_DIR_NAME: &str = "snippets";

const MANIFEST_FILE_NAME: &str = "spin-template.toml";

impl TemplateLayout {
    pub fn new(template_dir: impl AsRef<Path>) -> Self {
        Self {
            template_dir: template_dir.as_ref().to_owned(),
        }
    }

    pub fn metadata_dir(&self) -> PathBuf {
        self.template_dir.join(METADATA_DIR_NAME)
    }

    pub fn filters_dir(&self) -> PathBuf {
        self.metadata_dir().join(FILTERS_DIR_NAME)
    }

    pub fn filter_path(&self, filename: &str) -> PathBuf {
        self.filters_dir().join(filename)
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.metadata_dir().join(MANIFEST_FILE_NAME)
    }

    pub fn content_dir(&self) -> PathBuf {
        self.template_dir.join(CONTENT_DIR_NAME)
    }

    pub fn snippets_dir(&self) -> PathBuf {
        self.metadata_dir().join(SNIPPETS_DIR_NAME)
    }
}
