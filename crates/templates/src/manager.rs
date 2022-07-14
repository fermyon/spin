use std::path::Path;

use anyhow::Context;

use crate::{
    source::TemplateSource,
    store::{TemplateLayout, TemplateStore},
    template::Template,
};

/// Provides access to and operations on the set of installed
/// templates.
pub struct TemplateManager {
    store: TemplateStore,
}

/// Used during template installation to report progress and
/// current activity.
pub trait ProgressReporter {
    /// Report the specified message.
    fn report(&self, message: impl AsRef<str>);
}

/// Options controlling template installation.
#[derive(Debug)]
pub struct InstallOptions {
    exists_behaviour: ExistsBehaviour,
}

impl InstallOptions {
    /// Sets the option to update existing templates. If `update` is true,
    /// existing templates are updated. If false, existing templates are
    /// skipped.
    pub fn update(self, update: bool) -> Self {
        let exists_behaviour = if update {
            ExistsBehaviour::Update
        } else {
            ExistsBehaviour::Skip
        };

        Self { exists_behaviour }
    }
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            exists_behaviour: ExistsBehaviour::Skip,
        }
    }
}

#[derive(Debug)]
enum ExistsBehaviour {
    Skip,
    Update,
}

enum InstallationResult {
    Installed(Template),
    Skipped(String, SkippedReason),
}

/// The reason a template was skipped during installation.
pub enum SkippedReason {
    /// The template was skipped because it was already present.
    AlreadyExists,
    /// The template was skipped because its manifest was missing or invalid.
    InvalidManifest(String),
}

/// The results of installing a set of templates.
pub struct InstallationResults {
    /// The templates that were installed during the install operation.
    pub installed: Vec<Template>,
    /// The templates that were skipped during the install operation.
    pub skipped: Vec<(String, SkippedReason)>,
}

impl TemplateManager {
    /// Creates a `TemplateManager` for the default install location.
    pub fn default() -> anyhow::Result<Self> {
        let store = TemplateStore::default()?;
        Ok(Self { store })
    }

    /// Installs templates from the specified source.
    pub async fn install(
        &self,
        source: &TemplateSource,
        options: &InstallOptions,
        reporter: &impl ProgressReporter,
    ) -> anyhow::Result<InstallationResults> {
        if source.requires_copy() {
            reporter.report("Copying remote template source");
        }

        let local_source = source
            .get_local()
            .await
            .context("Failed to get template source")?;
        let template_dirs = local_source
            .template_directories()
            .await
            .context("Could not find templates in source")?;

        let mut installed = vec![];
        let mut skipped = vec![];

        for template_dir in template_dirs {
            let install_result = self
                .install_one(&template_dir, options, reporter)
                .await
                .with_context(|| {
                    format!("Failed to install template from {}", template_dir.display())
                })?;
            match install_result {
                InstallationResult::Installed(template) => installed.push(template),
                InstallationResult::Skipped(id, reason) => skipped.push((id, reason)),
            }
        }

        installed.sort_by_key(|t| t.id().to_owned());
        skipped.sort_by_key(|(id, _)| id.clone());

        Ok(InstallationResults { installed, skipped })
    }

    async fn install_one(
        &self,
        source_dir: &Path,
        options: &InstallOptions,
        reporter: &impl ProgressReporter,
    ) -> anyhow::Result<InstallationResult> {
        let layout = TemplateLayout::new(source_dir);
        let template = match Template::load_from(&layout) {
            Ok(t) => t,
            Err(e) => {
                let fake_id = source_dir
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("{}", source_dir.display()));
                let message = format!("{}", e);
                return Ok(InstallationResult::Skipped(
                    fake_id,
                    SkippedReason::InvalidManifest(message),
                ));
            }
        };
        let id = template.id();

        let message = format!("Installing template {}...", id);
        reporter.report(&message);

        let dest_dir = self.store.get_directory(&id);

        let template = if dest_dir.exists() {
            match options.exists_behaviour {
                ExistsBehaviour::Skip => {
                    return Ok(InstallationResult::Skipped(
                        id.to_owned(),
                        SkippedReason::AlreadyExists,
                    ))
                }
                ExistsBehaviour::Update => {
                    copy_template_over_existing(id, source_dir, &dest_dir).await?
                }
            }
        } else {
            copy_template_into(id, source_dir, &dest_dir).await?
        };

        Ok(InstallationResult::Installed(template))
    }

    /// Uninstalls the specified template.
    pub async fn uninstall(&self, template_id: impl AsRef<str>) -> anyhow::Result<()> {
        let template_dir = self.store.get_directory(template_id);
        tokio::fs::remove_dir_all(&template_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to delete template directory {}",
                    template_dir.display()
                )
            })
    }

    /// Lists all installed templates.
    pub async fn list(&self) -> anyhow::Result<Vec<Template>> {
        let mut templates = vec![];

        for template_layout in self.store.list_layouts().await? {
            let template = Template::load_from(&template_layout).with_context(|| {
                format!(
                    "Failed to read template from {}",
                    template_layout.metadata_dir().display()
                )
            })?;
            templates.push(template);
        }

        templates.sort_by_key(|t| t.id().to_owned());

        Ok(templates)
    }

    /// Gets the specified template. The result will be `Ok(Some(template))` if
    /// the template was found, and `Ok(None)` if the template was not
    /// found.
    pub fn get(&self, id: impl AsRef<str>) -> anyhow::Result<Option<Template>> {
        self.store
            .get_layout(id)
            .map(|l| Template::load_from(&l))
            .transpose()
    }
}

async fn copy_template_over_existing(
    id: &str,
    source_dir: &Path,
    dest_dir: &Path,
) -> anyhow::Result<Template> {
    // The nearby directory to which we initially copy the source
    let stage_dir = dest_dir.with_extension(".stage");
    // The nearby directory to which we move the existing
    let unstage_dir = dest_dir.with_extension(".unstage");

    // Clean up temp directories in case left over from previous failures.
    if stage_dir.exists() {
        tokio::fs::remove_dir_all(&stage_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed while deleting {} in order to update {}",
                    stage_dir.display(),
                    id
                )
            })?
    };

    if unstage_dir.exists() {
        tokio::fs::remove_dir_all(&unstage_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed while deleting {} in order to update {}",
                    unstage_dir.display(),
                    id
                )
            })?
    };

    // Copy template source into stage directory, and do best effort
    // cleanup if it goes wrong.
    let copy_to_stage_err = copy_template_into(id, source_dir, &stage_dir).await.err();
    if let Some(e) = copy_to_stage_err {
        let _ = tokio::fs::remove_dir_all(&stage_dir).await;
        return Err(e);
    };

    // We have a valid template in stage.  Now, move existing to unstage...
    if let Err(e) = tokio::fs::rename(dest_dir, &unstage_dir)
        .await
        .with_context(|| {
            format!(
                "Failed to move existing template out of {} in order to update {}",
                dest_dir.display(),
                id
            )
        })
    {
        let _ = tokio::fs::remove_dir_all(&stage_dir).await;
        return Err(e);
    }

    // ...and move stage into position.
    if let Err(e) = tokio::fs::rename(&stage_dir, dest_dir)
        .await
        .with_context(|| {
            format!(
                "Failed to move new template into {} in order to update {}",
                dest_dir.display(),
                id
            )
        })
    {
        // Put it back quick and hope nobody notices.
        let _ = tokio::fs::rename(&unstage_dir, dest_dir).await;
        let _ = tokio::fs::remove_dir_all(&stage_dir).await;
        return Err(e);
    }

    // Remove whichever directories remain.  (As we are ignoring errors, we
    // can skip checking whether the directories exist.)
    let _ = tokio::fs::remove_dir_all(&stage_dir).await;
    let _ = tokio::fs::remove_dir_all(&unstage_dir).await;

    load_template_from(id, dest_dir)
}

async fn copy_template_into(
    id: &str,
    source_dir: &Path,
    dest_dir: &Path,
) -> anyhow::Result<Template> {
    tokio::fs::create_dir_all(&dest_dir)
        .await
        .with_context(|| {
            format!(
                "Failed to create directory {} for {}",
                dest_dir.display(),
                id
            )
        })?;

    fs_extra::dir::copy(source_dir, &dest_dir, &copy_content()).with_context(|| {
        format!(
            "Failed to copy template content from {} to {} for {}",
            source_dir.display(),
            dest_dir.display(),
            id
        )
    })?;

    load_template_from(id, dest_dir)
}

fn load_template_from(id: &str, dest_dir: &Path) -> anyhow::Result<Template> {
    let layout = TemplateLayout::new(&dest_dir);
    Template::load_from(&layout).with_context(|| {
        format!(
            "Template {} was not copied correctly into {}",
            id,
            dest_dir.display()
        )
    })
}

fn copy_content() -> fs_extra::dir::CopyOptions {
    let mut options = fs_extra::dir::CopyOptions::new();
    options.content_only = true;
    options
}

impl InstallationResults {
    /// Gets whether the `InstallationResults` contains no templates. This
    /// indicates that no templates were found in the installation source.
    pub fn is_empty(&self) -> bool {
        self.installed.is_empty() && self.skipped.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use crate::RunOptions;

    use super::*;

    struct DiscardingReporter;

    impl ProgressReporter for DiscardingReporter {
        fn report(&self, _: impl AsRef<str>) {
            // Commit it then to the flames: for it can contain nothing but
            // sophistry and illusion.
        }
    }

    fn project_root() -> PathBuf {
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(crate_dir).join("..").join("..")
    }

    const TPLS_IN_THIS: usize = 8;

    #[tokio::test]
    async fn can_install_into_new_directory() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        assert_eq!(0, manager.list().await.unwrap().len());

        let install_result = manager
            .install(&source, &InstallOptions::default(), &DiscardingReporter)
            .await
            .unwrap();
        assert_eq!(TPLS_IN_THIS, install_result.installed.len());
        assert_eq!(0, install_result.skipped.len());

        assert_eq!(TPLS_IN_THIS, manager.list().await.unwrap().len());
    }

    #[tokio::test]
    async fn skips_bad_templates() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };

        let temp_source = tempdir().unwrap();
        let temp_source_tpls_dir = temp_source.path().join("templates");
        fs_extra::dir::copy(
            project_root().join("templates"),
            &temp_source_tpls_dir,
            &copy_content(),
        )
        .unwrap();
        fs::create_dir(temp_source_tpls_dir.join("notta-template")).unwrap();
        let source = TemplateSource::File(temp_source.path().to_owned());

        assert_eq!(0, manager.list().await.unwrap().len());

        let install_result = manager
            .install(&source, &InstallOptions::default(), &DiscardingReporter)
            .await
            .unwrap();
        assert_eq!(TPLS_IN_THIS, install_result.installed.len());
        assert_eq!(1, install_result.skipped.len());

        assert!(matches!(
            install_result.skipped[0].1,
            SkippedReason::InvalidManifest(_)
        ));
    }

    #[tokio::test]
    async fn can_uninstall() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(&source, &InstallOptions::default(), &DiscardingReporter)
            .await
            .unwrap();
        assert_eq!(TPLS_IN_THIS, manager.list().await.unwrap().len());
        manager.uninstall("http-rust").await.unwrap();

        let installed = manager.list().await.unwrap();
        assert_eq!(TPLS_IN_THIS - 1, installed.len());
        assert!(!installed.iter().any(|t| t.id() == "http-rust"));
    }

    #[tokio::test]
    async fn can_install_if_some_already_exist() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(&source, &InstallOptions::default(), &DiscardingReporter)
            .await
            .unwrap();
        manager.uninstall("http-rust").await.unwrap();
        manager.uninstall("http-go").await.unwrap();
        assert_eq!(TPLS_IN_THIS - 2, manager.list().await.unwrap().len());

        let install_result = manager
            .install(&source, &InstallOptions::default(), &DiscardingReporter)
            .await
            .unwrap();
        assert_eq!(2, install_result.installed.len());
        assert_eq!(TPLS_IN_THIS - 2, install_result.skipped.len());

        let installed = manager.list().await.unwrap();
        assert_eq!(TPLS_IN_THIS, installed.len());
        assert!(installed.iter().any(|t| t.id() == "http-rust"));
        assert!(installed.iter().any(|t| t.id() == "http-go"));
    }

    #[tokio::test]
    async fn can_update_existing() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(&source, &InstallOptions::default(), &DiscardingReporter)
            .await
            .unwrap();
        manager.uninstall("http-rust").await.unwrap();
        assert_eq!(TPLS_IN_THIS - 1, manager.list().await.unwrap().len());

        let install_result = manager
            .install(
                &source,
                &InstallOptions::default().update(true),
                &DiscardingReporter,
            )
            .await
            .unwrap();
        assert_eq!(TPLS_IN_THIS, install_result.installed.len());
        assert_eq!(0, install_result.skipped.len());

        let installed = manager.list().await.unwrap();
        assert_eq!(TPLS_IN_THIS, installed.len());
        assert!(installed.iter().any(|t| t.id() == "http-go"));
    }

    #[tokio::test]
    async fn can_read_installed_template() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(&source, &InstallOptions::default(), &DiscardingReporter)
            .await
            .unwrap();

        let template = manager.get("http-rust").unwrap().unwrap();
        assert_eq!(
            "HTTP request handler using Rust",
            template.description_or_empty()
        );

        let content_dir = template.content_dir().as_ref().unwrap();
        let cargo = tokio::fs::read_to_string(content_dir.join("Cargo.toml"))
            .await
            .unwrap();
        assert!(cargo.contains("name = \"{{project-name | kebab_case}}\""));
    }

    #[tokio::test]
    async fn can_run_template() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(&source, &InstallOptions::default(), &DiscardingReporter)
            .await
            .unwrap();

        let template = manager.get("http-rust").unwrap().unwrap();

        let dest_temp_dir = tempdir().unwrap();
        let output_dir = dest_temp_dir.path().join("myproj");
        let values = [
            ("project-description".to_owned(), "my desc".to_owned()),
            ("http-base".to_owned(), "/base".to_owned()),
            ("http-path".to_owned(), "/path/...".to_owned()),
        ]
        .into_iter()
        .collect();
        let options = RunOptions {
            output_path: output_dir.clone(),
            name: "my project".to_owned(),
            values,
        };

        template
            .run(options)
            .silent()
            .await
            .execute()
            .await
            .unwrap();

        let cargo = tokio::fs::read_to_string(output_dir.join("Cargo.toml"))
            .await
            .unwrap();
        assert!(cargo.contains("name = \"my-project\""));
    }
}
