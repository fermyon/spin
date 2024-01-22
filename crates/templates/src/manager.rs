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

#[allow(clippy::large_enum_variant)] // it's not worth it
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
    /// The template name does match the provided IDs
    IdNotIncluded,
}

/// The results of installing a set of templates.
pub struct InstallationResults {
    /// The templates that were installed during the install operation.
    pub installed: Vec<Template>,
    /// The templates that were skipped during the install operation.
    pub skipped: Vec<(String, SkippedReason)>,
}

/// The result of listing templates.
#[derive(Debug)]
pub struct ListResults {
    /// The installed templates.
    pub templates: Vec<Template>,
    /// Any warnings identified during the list operation.
    pub warnings: Vec<(String, InstalledTemplateWarning)>,
    /// Any skipped templates (populated as a result of filtering by tags).
    pub skipped: Vec<Template>,
}

impl ListResults {
    /// Returns true if no templates were found or skipped indicating that
    /// templates may not be installed.
    pub fn needs_install(&self) -> bool {
        self.templates.is_empty() && self.skipped.is_empty()
    }
}

/// A recoverable problem while listing templates.
#[derive(Debug)]
pub enum InstalledTemplateWarning {
    /// The manifest is invalid. The directory may not represent a template.
    InvalidManifest(String),
}

impl TemplateManager {
    /// Creates a `TemplateManager` for the default install location.
    pub fn try_default() -> anyhow::Result<Self> {
        let store = TemplateStore::try_default()?;
        Ok(Self::new(store))
    }

    pub(crate) fn new(store: TemplateStore) -> Self {
        Self { store }
    }

    /// Installs templates from the specified source.
    pub async fn install(
        &self,
        source: &TemplateSource,
        options: &InstallOptions,
        reporter: &impl ProgressReporter,
        install_template_list: &Vec<String>,
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
                .install_one(
                    &template_dir,
                    options,
                    source,
                    reporter,
                    install_template_list,
                )
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
        source: &TemplateSource,
        reporter: &impl ProgressReporter,
        install_template_list: &Vec<String>,
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

        if !install_template_list.is_empty() && !install_template_list.contains(&id.to_owned()) {
            let message = format!("Skipping template {}...", id);
            reporter.report(message);
            return Ok(InstallationResult::Skipped(
                id.to_owned(),
                SkippedReason::IdNotIncluded,
            ));
        }

        let message = format!("Installing template {}...", id);
        reporter.report(&message);

        let dest_dir = self.store.get_directory(id);

        let template = if dest_dir.exists() {
            match options.exists_behaviour {
                ExistsBehaviour::Skip => {
                    return Ok(InstallationResult::Skipped(
                        id.to_owned(),
                        SkippedReason::AlreadyExists,
                    ))
                }
                ExistsBehaviour::Update => {
                    copy_template_over_existing(id, source_dir, &dest_dir, source).await?
                }
            }
        } else {
            copy_template_into(id, source_dir, &dest_dir, source).await?
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
    pub async fn list(&self) -> anyhow::Result<ListResults> {
        let mut templates = vec![];
        let mut warnings = vec![];

        for template_layout in self.store.list_layouts().await? {
            match Template::load_from(&template_layout) {
                Ok(template) => templates.push(template),
                Err(e) => warnings.push(build_list_warning(&template_layout, e)?),
            }
        }

        templates.sort_by_key(|t| t.id().to_owned());

        Ok(ListResults {
            templates,
            warnings,
            skipped: vec![],
        })
    }

    /// Lists all installed templates that match all the provided tags.
    pub async fn list_with_tags(&self, tags: &[String]) -> anyhow::Result<ListResults> {
        let ListResults {
            templates,
            warnings,
            ..
        } = self.list().await?;

        let (templates, skipped) = templates
            .into_iter()
            .partition(|tpl| tpl.matches_all_tags(tags));

        Ok(ListResults {
            templates,
            warnings,
            skipped,
        })
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
    source: &TemplateSource,
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
    let copy_to_stage_err = copy_template_into(id, source_dir, &stage_dir, source)
        .await
        .err();
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
    source: &TemplateSource,
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

    fs_extra::dir::copy(source_dir, dest_dir, &copy_content()).with_context(|| {
        format!(
            "Failed to copy template content from {} to {} for {}",
            source_dir.display(),
            dest_dir.display(),
            id
        )
    })?;

    write_install_record(dest_dir, source);

    load_template_from(id, dest_dir)
}

fn write_install_record(dest_dir: &Path, source: &TemplateSource) {
    let layout = TemplateLayout::new(dest_dir);
    let install_record_path = layout.installation_record_file();

    // A failure here shouldn't fail the install
    let install_record = source.to_install_record();
    if let Ok(record_text) = toml::to_string_pretty(&install_record) {
        _ = std::fs::write(install_record_path, record_text);
    }
}

fn load_template_from(id: &str, dest_dir: &Path) -> anyhow::Result<Template> {
    let layout = TemplateLayout::new(dest_dir);
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

fn build_list_warning(
    template_layout: &TemplateLayout,
    load_err: anyhow::Error,
) -> anyhow::Result<(String, InstalledTemplateWarning)> {
    match template_layout.metadata_dir().parent() {
        Some(source_dir) => {
            let fake_id = source_dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{}", source_dir.display()));
            let message = format!("{}", load_err);
            Ok((fake_id, InstalledTemplateWarning::InvalidManifest(message)))
        }
        None => Err(load_err).context("Failed to load template but unable to determine which one"),
    }
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
    use std::{collections::HashMap, fs, path::PathBuf};

    use tempfile::tempdir;

    use crate::{RunOptions, TemplateVariantInfo};

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

    fn test_data_root() -> PathBuf {
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(crate_dir).join("tests")
    }

    const TPLS_IN_THIS: usize = 12;

    #[tokio::test]
    async fn can_install_into_new_directory() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        assert_eq!(0, manager.list().await.unwrap().templates.len());

        let install_result = manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();
        assert_eq!(TPLS_IN_THIS, install_result.installed.len());
        assert_eq!(0, install_result.skipped.len());

        assert_eq!(TPLS_IN_THIS, manager.list().await.unwrap().templates.len());
        assert_eq!(0, manager.list().await.unwrap().warnings.len());
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

        assert_eq!(0, manager.list().await.unwrap().templates.len());
        assert_eq!(0, manager.list().await.unwrap().warnings.len());

        let install_result = manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
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
    async fn can_list_all_templates_with_empty_tags() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let list_results = manager.list_with_tags(&[]).await.unwrap();
        assert_eq!(0, list_results.skipped.len());
        assert_eq!(TPLS_IN_THIS, list_results.templates.len());
    }

    #[tokio::test]
    async fn skips_when_all_tags_do_not_match() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let tags_to_match = vec!["c".to_string(), "unused_tag".to_string()];

        let list_results = manager.list_with_tags(&tags_to_match).await.unwrap();
        assert_eq!(TPLS_IN_THIS, list_results.skipped.len());
        assert_eq!(0, list_results.templates.len());
    }

    #[tokio::test]
    async fn can_list_templates_with_multiple_tags() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let tags_to_match = vec!["http".to_string(), "c".to_string()];

        let list_results = manager.list_with_tags(&tags_to_match).await.unwrap();
        assert_eq!(TPLS_IN_THIS - 1, list_results.skipped.len());
        assert_eq!(1, list_results.templates.len());
    }

    #[tokio::test]
    async fn can_list_templates_with_case_insensitive_tags() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let list_results = manager.list_with_tags(&["C".to_string()]).await.unwrap();
        assert_eq!(TPLS_IN_THIS - 1, list_results.skipped.len());
        assert_eq!(1, list_results.templates.len());
    }

    #[tokio::test]
    async fn can_skip_templates_with_missing_tag() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let list_results = manager
            .list_with_tags(&["unused_tag".to_string()])
            .await
            .unwrap();
        assert_eq!(TPLS_IN_THIS, list_results.skipped.len());
        assert_eq!(0, list_results.templates.len());
    }

    #[tokio::test]
    async fn can_list_if_bad_dir_in_store() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        assert_eq!(0, manager.list().await.unwrap().templates.len());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        assert_eq!(TPLS_IN_THIS, manager.list().await.unwrap().templates.len());
        assert_eq!(0, manager.list().await.unwrap().warnings.len());

        fs::create_dir(temp_dir.path().join("i-trip-you-up")).unwrap();

        let list_results = manager.list().await.unwrap();
        assert_eq!(TPLS_IN_THIS, list_results.templates.len());
        assert_eq!(1, list_results.warnings.len());
        assert_eq!("i-trip-you-up", list_results.warnings[0].0);
        assert!(matches!(
            list_results.warnings[0].1,
            InstalledTemplateWarning::InvalidManifest(_)
        ));
    }

    #[tokio::test]
    async fn can_uninstall() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();
        assert_eq!(TPLS_IN_THIS, manager.list().await.unwrap().templates.len());
        manager.uninstall("http-rust").await.unwrap();

        let installed = manager.list().await.unwrap();
        assert_eq!(TPLS_IN_THIS - 1, installed.templates.len());
        assert_eq!(0, installed.warnings.len());
        assert!(!installed.templates.iter().any(|t| t.id() == "http-rust"));
    }

    #[tokio::test]
    async fn can_install_if_some_already_exist() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();
        manager.uninstall("http-rust").await.unwrap();
        manager.uninstall("http-go").await.unwrap();
        assert_eq!(
            TPLS_IN_THIS - 2,
            manager.list().await.unwrap().templates.len()
        );

        let install_result = manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();
        assert_eq!(2, install_result.installed.len());
        assert_eq!(TPLS_IN_THIS - 2, install_result.skipped.len());

        let installed = manager.list().await.unwrap().templates;
        assert_eq!(TPLS_IN_THIS, installed.len());
        assert!(installed.iter().any(|t| t.id() == "http-rust"));
        assert!(installed.iter().any(|t| t.id() == "http-go"));
    }

    #[tokio::test]
    async fn can_install_only_some() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        let install_result = manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &vec!["http-rust".into(), "http-c".into()],
            )
            .await
            .unwrap();
        assert_eq!(2, install_result.installed.len());
        assert_eq!(TPLS_IN_THIS - 2, install_result.skipped.len());

        let installed = manager.list().await.unwrap().templates;
        assert!(installed.iter().any(|t| t.id() == "http-rust"));
        assert!(installed.iter().any(|t| t.id() == "http-c"));
    }

    #[tokio::test]
    async fn can_update_existing() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();
        manager.uninstall("http-rust").await.unwrap();
        assert_eq!(
            TPLS_IN_THIS - 1,
            manager.list().await.unwrap().templates.len()
        );

        let install_result = manager
            .install(
                &source,
                &InstallOptions::default().update(true),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();
        assert_eq!(TPLS_IN_THIS, install_result.installed.len());
        assert_eq!(0, install_result.skipped.len());

        let installed = manager.list().await.unwrap().templates;
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
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
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
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let template = manager.get("http-rust").unwrap().unwrap();

        let dest_temp_dir = tempdir().unwrap();
        let output_dir = dest_temp_dir.path().join("myproj");
        let values = [
            ("project-description".to_owned(), "my desc".to_owned()),
            ("http-path".to_owned(), "/path/...".to_owned()),
        ]
        .into_iter()
        .collect();
        let options = RunOptions {
            variant: crate::template::TemplateVariantInfo::NewApplication,
            output_path: output_dir.clone(),
            name: "my project".to_owned(),
            values,
            accept_defaults: false,
        };

        template.run(options).silent().await.unwrap();

        let cargo = tokio::fs::read_to_string(output_dir.join("Cargo.toml"))
            .await
            .unwrap();
        assert!(cargo.contains("name = \"my-project\""));
    }

    #[tokio::test]
    async fn can_run_template_with_accept_defaults() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let template = manager.get("http-rust").unwrap().unwrap();

        let dest_temp_dir = tempdir().unwrap();
        let output_dir = dest_temp_dir.path().join("myproj");
        let values = HashMap::new();
        let options = RunOptions {
            variant: crate::template::TemplateVariantInfo::NewApplication,
            output_path: output_dir.clone(),
            name: "my project".to_owned(),
            values,
            accept_defaults: true,
        };

        template.run(options).silent().await.unwrap();

        let cargo = tokio::fs::read_to_string(output_dir.join("Cargo.toml"))
            .await
            .unwrap();
        assert!(cargo.contains("name = \"my-project\""));
        let spin_toml = tokio::fs::read_to_string(output_dir.join("spin.toml"))
            .await
            .unwrap();
        assert!(spin_toml.contains("route = \"/...\""));
    }

    #[tokio::test]
    async fn cannot_use_custom_filter_in_template() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(test_data_root());

        let install_results = manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        assert_eq!(1, install_results.skipped.len());

        let (id, reason) = &install_results.skipped[0];
        assert_eq!("testing-custom-filter", id);
        let SkippedReason::InvalidManifest(message) = reason else {
            panic!("skip reason should be InvalidManifest"); // clippy dislikes assert!(false...)
        };
        assert_contains(message, "filters");
        assert_contains(message, "not supported");
    }

    #[tokio::test]
    async fn can_add_component_from_template() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let dest_temp_dir = tempdir().unwrap();
        let application_dir = dest_temp_dir.path().join("multi");

        // Set up the containing app
        {
            let template = manager.get("http-empty").unwrap().unwrap();

            let values = [("project-description".to_owned(), "my desc".to_owned())]
                .into_iter()
                .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::NewApplication,
                output_path: application_dir.clone(),
                name: "my multi project".to_owned(),
                values,
                accept_defaults: false,
            };

            template.run(options).silent().await.unwrap();
        }

        let spin_toml_path = application_dir.join("spin.toml");
        assert!(spin_toml_path.exists(), "expected spin.toml to be created");

        // Now add a component
        {
            let template = manager.get("http-rust").unwrap().unwrap();

            let output_dir = "hello";
            let values = [
                ("project-description".to_owned(), "hello".to_owned()),
                ("http-path".to_owned(), "/hello".to_owned()),
            ]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::AddComponent {
                    manifest_path: spin_toml_path.clone(),
                },
                output_path: PathBuf::from(output_dir),
                name: "hello".to_owned(),
                values,
                accept_defaults: false,
            };

            template.run(options).silent().await.unwrap();
        }

        // And another
        {
            let template = manager.get("http-rust").unwrap().unwrap();

            let output_dir = "encore";
            let values = [
                ("project-description".to_owned(), "hello 2".to_owned()),
                ("http-path".to_owned(), "/hello-2".to_owned()),
            ]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::AddComponent {
                    manifest_path: spin_toml_path.clone(),
                },
                output_path: PathBuf::from(output_dir),
                name: "hello 2".to_owned(),
                values,
                accept_defaults: false,
            };

            template.run(options).silent().await.unwrap();
        }

        let cargo1 = tokio::fs::read_to_string(application_dir.join("hello/Cargo.toml"))
            .await
            .unwrap();
        assert!(cargo1.contains("name = \"hello\""));

        let cargo2 = tokio::fs::read_to_string(application_dir.join("encore/Cargo.toml"))
            .await
            .unwrap();
        assert!(cargo2.contains("name = \"hello-2\""));

        let spin_toml = tokio::fs::read_to_string(&spin_toml_path).await.unwrap();
        assert!(spin_toml.contains("source = \"hello/target/wasm32-wasi/release/hello.wasm\""));
        assert!(spin_toml.contains("source = \"encore/target/wasm32-wasi/release/hello_2.wasm\""));
    }

    #[tokio::test]
    async fn can_add_variables_from_template() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source1 = TemplateSource::File(test_data_root());
        let source2 = TemplateSource::File(project_root()); // We will need some of the standard templates too

        manager
            .install(
                &source1,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();
        manager
            .install(
                &source2,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let dest_temp_dir = tempdir().unwrap();
        let application_dir = dest_temp_dir.path().join("spinvars");

        // Set up the containing app
        {
            let template = manager.get("http-rust").unwrap().unwrap();

            let values = [
                ("project-description".to_owned(), "my desc".to_owned()),
                ("http-path".to_owned(), "/...".to_owned()),
            ]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::NewApplication,
                output_path: application_dir.clone(),
                name: "my various project".to_owned(),
                values,
                accept_defaults: false,
            };

            template.run(options).silent().await.unwrap();
        }

        let spin_toml_path = application_dir.join("spin.toml");
        assert!(spin_toml_path.exists(), "expected spin.toml to be created");

        // Now add the variables
        {
            let template = manager.get("add-variables").unwrap().unwrap();

            let output_dir = "hello";
            let values = [(
                "service-url".to_owned(),
                "https://service.example.com".to_owned(),
            )]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::AddComponent {
                    manifest_path: spin_toml_path.clone(),
                },
                output_path: PathBuf::from(output_dir),
                name: "insertvars".to_owned(),
                values,
                accept_defaults: false,
            };

            template.run(options).silent().await.unwrap();
        }

        let spin_toml = tokio::fs::read_to_string(&spin_toml_path).await.unwrap();

        assert!(spin_toml.contains("[variables]\nsecret"));
        assert!(spin_toml.contains("url = { default = \"https://service.example.com\" }"));

        assert!(spin_toml.contains("[component.insertvars]"));
        assert!(spin_toml.contains("[component.insertvars.variables]"));
        assert!(spin_toml.contains("kv_credentials = \"{{ secret }}\""));
    }

    #[tokio::test]
    async fn can_overwrite_existing_variables() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source1 = TemplateSource::File(test_data_root());
        let source2 = TemplateSource::File(project_root()); // We will need some of the standard templates too

        manager
            .install(
                &source1,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();
        manager
            .install(
                &source2,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let dest_temp_dir = tempdir().unwrap();
        let application_dir = dest_temp_dir.path().join("spinvars");

        // Set up the containing app
        {
            let template = manager.get("http-rust").unwrap().unwrap();

            let values = [
                ("project-description".to_owned(), "my desc".to_owned()),
                ("http-path".to_owned(), "/...".to_owned()),
            ]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::NewApplication,
                output_path: application_dir.clone(),
                name: "my various project".to_owned(),
                values,
                accept_defaults: false,
            };

            template.run(options).silent().await.unwrap();
        }

        let spin_toml_path = application_dir.join("spin.toml");
        assert!(spin_toml_path.exists(), "expected spin.toml to be created");

        // Now add the variables
        {
            let template = manager.get("add-variables").unwrap().unwrap();

            let output_dir = "hello";
            let values = [(
                "service-url".to_owned(),
                "https://service.example.com".to_owned(),
            )]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::AddComponent {
                    manifest_path: spin_toml_path.clone(),
                },
                output_path: PathBuf::from(output_dir),
                name: "insertvars".to_owned(),
                values,
                accept_defaults: false,
            };

            template.run(options).silent().await.unwrap();
        }

        // Now add them again but with different values
        {
            let template = manager.get("add-variables").unwrap().unwrap();

            let output_dir = "hello";
            let values = [(
                "service-url".to_owned(),
                "https://other.example.com".to_owned(),
            )]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::AddComponent {
                    manifest_path: spin_toml_path.clone(),
                },
                output_path: PathBuf::from(output_dir),
                name: "insertvarsagain".to_owned(),
                values,
                accept_defaults: false,
            };

            template.run(options).silent().await.unwrap();
        }

        let spin_toml = tokio::fs::read_to_string(&spin_toml_path).await.unwrap();
        assert!(spin_toml.contains("url = { default = \"https://other.example.com\" }"));
        assert!(!spin_toml.contains("service.example.com"));
    }

    #[tokio::test]
    async fn cannot_add_component_that_does_not_match_trigger() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let dest_temp_dir = tempdir().unwrap();
        let application_dir = dest_temp_dir.path().join("multi");

        // Set up the containing app
        {
            let template = manager.get("redis-rust").unwrap().unwrap();

            let values = [
                ("project-description".to_owned(), "my desc".to_owned()),
                (
                    "redis-address".to_owned(),
                    "redis://localhost:6379".to_owned(),
                ),
                (
                    "redis-channel".to_owned(),
                    "the-horrible-knuckles".to_owned(),
                ),
            ]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::NewApplication,
                output_path: application_dir.clone(),
                name: "my multi project".to_owned(),
                values,
                accept_defaults: false,
            };

            template.run(options).silent().await.unwrap();
        }

        let spin_toml_path = application_dir.join("spin.toml");
        assert!(spin_toml_path.exists(), "expected spin.toml to be created");

        // Now add a component
        {
            let template = manager.get("http-rust").unwrap().unwrap();

            let output_dir = "hello";
            let values = [
                ("project-description".to_owned(), "hello".to_owned()),
                ("http-path".to_owned(), "/hello".to_owned()),
            ]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::AddComponent {
                    manifest_path: spin_toml_path.clone(),
                },
                output_path: PathBuf::from(output_dir),
                name: "hello".to_owned(),
                values,
                accept_defaults: false,
            };

            template
                .run(options)
                .silent()
                .await
                .expect_err("Expected to fail to add component, but it succeeded");
        }
    }

    #[tokio::test]
    async fn cannot_add_component_that_does_not_match_manifest() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(project_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let dest_temp_dir = tempdir().unwrap();
        let application_dir = dest_temp_dir.path().join("multi");

        // Set up the containing app
        {
            let fake_v1_src = test_data_root().join("v1manifest.toml");
            let fake_v1_dest = application_dir.join("spin.toml");
            tokio::fs::create_dir_all(&application_dir).await.unwrap();
            tokio::fs::copy(fake_v1_src, fake_v1_dest).await.unwrap();
        }

        let spin_toml_path = application_dir.join("spin.toml");
        assert!(
            spin_toml_path.exists(),
            "expected v1 spin.toml to be created"
        );

        // Now add a component
        {
            let template = manager.get("http-rust").unwrap().unwrap();

            let output_dir = "hello";
            let values = [
                ("project-description".to_owned(), "hello".to_owned()),
                ("http-path".to_owned(), "/hello".to_owned()),
            ]
            .into_iter()
            .collect();
            let options = RunOptions {
                variant: crate::template::TemplateVariantInfo::AddComponent {
                    manifest_path: spin_toml_path.clone(),
                },
                output_path: PathBuf::from(output_dir),
                name: "hello".to_owned(),
                values,
                accept_defaults: false,
            };

            template
                .run(options)
                .silent()
                .await
                .expect_err("Expected to fail to add component, but it succeeded");
        }
    }

    #[tokio::test]
    async fn cannot_new_a_component_only_template() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source1 = TemplateSource::File(test_data_root());
        let source2 = TemplateSource::File(project_root());

        manager
            .install(
                &source1,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();
        manager
            .install(
                &source2,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let dummy_dir = temp_dir.path().join("dummy");
        let manifest_path = dummy_dir.join("ignored_spin.toml");
        let add_component = TemplateVariantInfo::AddComponent { manifest_path };

        let redirect = manager.get("add-only-redirect").unwrap().unwrap();
        assert!(!redirect.supports_variant(&TemplateVariantInfo::NewApplication));
        assert!(redirect.supports_variant(&add_component));

        let http_rust = manager.get("http-rust").unwrap().unwrap();
        assert!(http_rust.supports_variant(&TemplateVariantInfo::NewApplication));
        assert!(http_rust.supports_variant(&add_component));

        let http_empty = manager.get("http-empty").unwrap().unwrap();
        assert!(http_empty.supports_variant(&TemplateVariantInfo::NewApplication));
        assert!(!http_empty.supports_variant(&add_component));
    }

    #[tokio::test]
    async fn fails_on_unknown_filter() {
        let temp_dir = tempdir().unwrap();
        let store = TemplateStore::new(temp_dir.path());
        let manager = TemplateManager { store };
        let source = TemplateSource::File(test_data_root());

        manager
            .install(
                &source,
                &InstallOptions::default(),
                &DiscardingReporter,
                &Vec::new(),
            )
            .await
            .unwrap();

        let template = manager.get("bad-non-existent-filter").unwrap().unwrap();

        let dest_temp_dir = tempdir().unwrap();
        let output_dir = dest_temp_dir.path().join("myproj");
        let values = [("p1".to_owned(), "biscuits".to_owned())]
            .into_iter()
            .collect();
        let options = RunOptions {
            variant: crate::template::TemplateVariantInfo::NewApplication,
            output_path: output_dir.clone(),
            name: "bad-filter-should-fail ".to_owned(),
            values,
            accept_defaults: false,
        };

        let err = template
            .run(options)
            .silent()
            .await
            .expect_err("Expected template to fail but it passed");

        let err_str = err.to_string();

        assert_contains(&err_str, "internal error");
        assert_contains(&err_str, "unknown filter 'lol_snort'");
    }

    fn assert_contains(actual: &str, expected: &str) {
        assert!(
            actual.contains(expected),
            "expected string containing '{expected}' but got '{actual}'"
        );
    }
}
