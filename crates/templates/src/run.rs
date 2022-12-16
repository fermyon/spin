use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use itertools::Itertools;
use path_absolutize::Absolutize;
use walkdir::WalkDir;

use crate::{
    cancellable::Cancellable,
    interaction::{InteractionStrategy, Interactive, Silent},
    template::TemplateVariantInfo,
};
use crate::{
    renderer::{RenderOperation, TemplateContent, TemplateRenderer},
    template::Template,
};

/// Executes a template to the point where it is ready to generate
/// artefacts.
pub struct Run {
    pub(crate) template: Template,
    pub(crate) options: RunOptions,
}

/// Options controlling the execution of a template.
pub struct RunOptions {
    /// The variant mode in which to run the template.
    pub variant: TemplateVariantInfo,
    /// The name of the generated item.
    pub name: String,
    /// The path at which to generate artefacts.
    pub output_path: PathBuf,
    /// The values to use for template parameters.
    pub values: HashMap<String, String>,
    /// If true accept default values where available
    pub accept_defaults: bool,
}

impl Run {
    pub(crate) fn new(template: Template, options: RunOptions) -> Self {
        Self { template, options }
    }

    /// Runs the template interactively. The user will be prompted for any
    /// information or input the template needs, such as parameter values.
    /// Execution will block while waiting on user responses.
    pub async fn interactive(&self) -> anyhow::Result<()> {
        self.run(Interactive).await
    }

    /// Runs the template silently. The template will be executed without
    /// user interaction, and will not wait on the user. If the template needs
    /// any information or input that was not provided in the `RunOptions`,
    /// execution will fail and result in an error.
    pub async fn silent(&self) -> anyhow::Result<()> {
        self.run(Silent).await
    }

    async fn run(&self, interaction: impl InteractionStrategy) -> anyhow::Result<()> {
        self.build_renderer(interaction)
            .await
            .and_then(|t| t.render())
            .and_then_async(|o| async move { o.write().await })
            .await
            .err()
    }

    async fn build_renderer(
        &self,
        interaction: impl InteractionStrategy,
    ) -> Cancellable<TemplateRenderer, anyhow::Error> {
        self.build_renderer_raw(interaction).await.into()
    }

    // The 'raw' in this refers to the output type, which is an ugly representation
    // of cancellation: Ok(Some(...)) means a result, Ok(None) means cancelled, Err
    // means error. Why have this ugly representation? Because it makes it terser to
    // write using the Rust `?` operator to early-return. It would be lovely to find
    // a better way but I don't see one yet...
    async fn build_renderer_raw(
        &self,
        interaction: impl InteractionStrategy,
    ) -> anyhow::Result<Option<TemplateRenderer>> {
        self.validate_trigger().await?;

        // TODO: rationalise `path` and `dir`
        let to = self.generation_target_dir();

        match interaction.allow_generate_into(&to) {
            Cancellable::Cancelled => return Ok(None),
            Cancellable::Ok(_) => (),
            Cancellable::Err(e) => return Err(e),
        };

        self.validate_provided_values()?;

        let files = match self.template.content_dir() {
            None => vec![],
            Some(path) => {
                let from = path
                    .absolutize()
                    .context("Failed to get absolute path of template directory")?;
                self.included_files(&from, &to)?
            }
        };

        let snippets = self
            .template
            .snippets(&self.options.variant)
            .iter()
            .map(|(id, path)| self.snippet_operation(id, path))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let render_operations = files.into_iter().chain(snippets).collect();

        match interaction.populate_parameters(self) {
            Cancellable::Ok(parameter_values) => {
                let values = self
                    .special_values()
                    .await
                    .into_iter()
                    .chain(parameter_values)
                    .collect();
                let prepared_template = TemplateRenderer {
                    render_operations,
                    parameter_values: values,
                };
                Ok(Some(prepared_template))
            }
            Cancellable::Cancelled => Ok(None),
            Cancellable::Err(e) => Err(e),
        }
    }

    fn included_files(&self, from: &Path, to: &Path) -> anyhow::Result<Vec<RenderOperation>> {
        let all_content_files = Self::list_content_files(from)?;
        let included_files =
            self.template
                .included_files(from, all_content_files, &self.options.variant);
        let template_contents = self.read_all(included_files)?;
        let outputs = Self::to_output_paths(from, to, template_contents);
        let file_ops = outputs
            .into_iter()
            .map(|(path, content)| RenderOperation::WriteFile(path, content))
            .collect();
        Ok(file_ops)
    }

    async fn special_values(&self) -> HashMap<String, String> {
        let mut values = HashMap::new();

        let authors = crate::environment::get_authors().await.unwrap_or_default();
        values.insert("authors".into(), authors.author);
        values.insert("username".into(), authors.username);
        values.insert("project-name".into(), self.options.name.clone());
        values.insert(
            "output-path".into(),
            self.relative_target_dir().to_string_lossy().to_string(),
        );

        values
    }

    fn relative_target_dir(&self) -> &Path {
        &self.options.output_path
    }

    fn generation_target_dir(&self) -> PathBuf {
        match &self.options.variant {
            TemplateVariantInfo::NewApplication => self.options.output_path.clone(),
            TemplateVariantInfo::AddComponent { manifest_path } => manifest_path
                .parent()
                .unwrap()
                .join(&self.options.output_path),
        }
    }

    fn validate_provided_values(&self) -> anyhow::Result<()> {
        let errors = self
            .options
            .values
            .iter()
            .filter_map(|(n, v)| self.validate_value(n, v))
            .collect_vec();
        if errors.is_empty() {
            Ok(())
        } else {
            // TODO: better to provide this as a structured object and let the caller choose how to present it
            let errors_msg = errors.iter().map(|s| format!("- {}", s)).join("\n");
            Err(anyhow!(
                "The following provided value(s) are invalid according to the template:\n{}",
                errors_msg
            ))
        }
    }

    fn validate_value(&self, name: &str, value: &str) -> Option<String> {
        match self.template.parameter(name) {
            None => Some(format!(
                "Template does not contain a parameter named '{}'",
                name
            )),
            Some(p) => match p.validate_value(value) {
                Ok(_) => None,
                Err(e) => Some(format!("{}: {}", name, e)),
            },
        }
    }

    async fn validate_trigger(&self) -> anyhow::Result<()> {
        match &self.options.variant {
            TemplateVariantInfo::NewApplication => Ok(()),
            TemplateVariantInfo::AddComponent { manifest_path } => {
                match crate::app_info::AppInfo::from_file(manifest_path) {
                    Some(Ok(app_info)) => self
                        .template
                        .check_compatible_trigger(app_info.trigger_type()),
                    _ => Ok(()), // Fail forgiving - don't block the user if things are under construction
                }
            }
        }
    }

    fn snippet_operation(&self, id: &str, snippet_file: &str) -> anyhow::Result<RenderOperation> {
        let snippets_dir = self
            .template
            .snippets_dir()
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Template snippets directory not found"))?;
        let abs_snippet_file = snippets_dir.join(snippet_file);
        let file_content = std::fs::read(abs_snippet_file)
            .with_context(|| format!("Error reading snippet file {}", snippet_file))?;
        let content = TemplateContent::infer_from_bytes(file_content, &self.template_parser())
            .with_context(|| format!("Error parsing snippet file {}", snippet_file))?;

        match id {
            "component" => {
                match &self.options.variant {
                    TemplateVariantInfo::AddComponent { manifest_path } =>
                        Ok(RenderOperation::AppendToml(
                            manifest_path.clone(),
                            content,
                        )),
                    TemplateVariantInfo::NewApplication =>
                        Err(anyhow::anyhow!("Spin doesn't know what to do with a 'component' snippet outside an 'add component' operation")),
                }
            },
            _ => Err(anyhow::anyhow!(
                "Spin doesn't know what to do with snippet {}",
                id
            )),
        }
    }

    fn list_content_files(from: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let walker = WalkDir::new(from);
        let files = walker
            .into_iter()
            .filter_map(|entry| match entry {
                Err(e) => Some(Err(e)),
                Ok(de) => {
                    if de.file_type().is_file() {
                        Some(Ok(de.path().to_owned()))
                    } else {
                        None
                    }
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(files)
    }

    // TODO: async when we know where things sit
    fn read_all(&self, paths: Vec<PathBuf>) -> anyhow::Result<Vec<(PathBuf, TemplateContent)>> {
        let template_parser = self.template_parser();
        let contents = paths
            .iter()
            .map(std::fs::read)
            .map(|c| {
                c.map_err(|e| e.into())
                    .and_then(|cc| TemplateContent::infer_from_bytes(cc, &template_parser))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let pairs = paths.into_iter().zip(contents).collect();
        Ok(pairs)
    }

    fn to_output_paths<T>(
        src_dir: &Path,
        dest_dir: &Path,
        contents: Vec<(PathBuf, T)>,
    ) -> Vec<(PathBuf, T)> {
        contents
            .into_iter()
            .filter_map(|f| Self::to_output_path(src_dir, dest_dir, f))
            .collect()
    }

    fn to_output_path<T>(
        src_dir: &Path,
        dest_dir: &Path,
        (source, cont): (PathBuf, T),
    ) -> Option<(PathBuf, T)> {
        pathdiff::diff_paths(source, src_dir).map(|rel| (dest_dir.join(rel), cont))
    }

    fn template_parser(&self) -> liquid::Parser {
        let mut builder = liquid::ParserBuilder::with_stdlib()
            .filter(crate::filters::KebabCaseFilterParser)
            .filter(crate::filters::PascalCaseFilterParser)
            .filter(crate::filters::SnakeCaseFilterParser)
            .filter(crate::filters::HttpWildcardFilterParser);
        for filter in self.template.custom_filters() {
            builder = builder.filter(filter);
        }
        builder
            .build()
            .expect("can't fail due to no partials support")
    }
}
