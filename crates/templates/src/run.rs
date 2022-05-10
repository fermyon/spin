use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use itertools::Itertools;
use path_absolutize::Absolutize;
use walkdir::WalkDir;

use crate::template::{Template, TemplateParameter};

/// Executes a template to the point where it is ready to generate
/// artefacts.
pub struct Run {
    template: Template,
    options: RunOptions,
}

/// Options controlling the execution of a template.
pub struct RunOptions {
    /// The name of the generated item.
    pub name: Option<String>,
    /// The path at which to generate artefacts.
    pub output_path: Option<PathBuf>,
    /// The values to use for template parameters.
    pub values: HashMap<String, String>,
}

enum Cancellable<T, E> {
    Cancelled,
    Err(E),
    Ok(T),
}

impl<T, E> Cancellable<T, E> {
    fn from_result_option(ro: Result<Option<T>, E>) -> Self {
        match ro {
            Ok(Some(t)) => Self::Ok(t),
            Ok(None) => Self::Cancelled,
            Err(e) => Self::Err(e),
        }
    }
}

/// The result of running a template to the point where it
/// is ready to write its outputs.
pub struct TemplatePreparationResult {
    // Ok(None) means the run was cancelled - TODO: consider a Cancellable
    // enum to be more self-documenting
    // inner: anyhow::Result<Option<(HashMap<PathBuf, Vec<u8>>, HashMap<String, String>)>>,
    inner: Cancellable<PreparedTemplate, anyhow::Error>,
}

struct PreparedTemplate {
    files: HashMap<PathBuf, TemplateContent>,
    parameter_values: HashMap<String, String>,
}

enum TemplateContent {
    Template(liquid::Template),
    Binary(Vec<u8>),
}

struct TemplateOutputs {
    files: HashMap<PathBuf, Vec<u8>>,
}

impl Run {
    pub(crate) fn new(template: Template, options: RunOptions) -> Self {
        Self { template, options }
    }

    /// Runs the template interactively. The user will be prompted for any
    /// information or input the template needs, such as parameter values.
    /// Execution will block while waiting on user responses.
    ///
    /// This function runs the template to the point where it is ready to
    /// write artefacts to the output. You must still call `execute` on the
    /// result to perform the write.
    pub fn interactive(&self) -> TemplatePreparationResult {
        let raw_prepared = self.run_inner(|| self.populate_parameters_interactive());
        let inner = Cancellable::from_result_option(raw_prepared);
        TemplatePreparationResult { inner }
    }

    /// Runs the template silently. The template will be executed without
    /// user interaction, and will not wait on the user. If the template needs
    /// any information or input that was not provided in the `RunOptions`,
    /// execution will fail and result in an error.
    ///
    /// This function runs the template to the point where it is ready to
    /// write artefacts to the output. You must still call `execute` on the
    /// result to perform the write.
    pub fn silent(&self) -> TemplatePreparationResult {
        let raw_prepared = self.run_inner(|| self.populate_parameters_silent());
        let inner = Cancellable::from_result_option(raw_prepared);
        TemplatePreparationResult { inner }
    }

    fn run_inner(
        &self,
        populate_parameters: impl Fn() -> anyhow::Result<Option<HashMap<String, String>>>,
    ) -> anyhow::Result<Option<PreparedTemplate>> {
        // TODO: Ok(None) means the run was cancelled - this is hard to follow but plays
        // nicely with the Rust ? operator - is there a better way?

        self.validate_provided_values()?;

        // TODO: rationalise `path` and `dir`
        let to = match &self.options.output_path {
            None => std::env::current_dir()?, // TODO: handle error
            Some(path) => path.clone(),
        };

        let outputs = match self.template.content_dir() {
            None => HashMap::new(),
            Some(path) => {
                let from = path
                    .absolutize()
                    .context("Failed to get absoluate path of template directory")?
                    .into_owned();
                let template_content_files = Self::collect_all_content(&from)?;
                // TODO: okay we do want to do *some* parsing here because we don't want
                // to prompt if the template bodies are garbage
                let template_contents = Self::read_all(template_content_files)?;
                Self::to_output_paths(&from, &to, template_contents)
            }
        };

        match populate_parameters()? {
            Some(parameter_values) => {
                // let outputs = Self::render_all(output_templates, &parameter_values)?;
                let prepared_template = PreparedTemplate {
                    files: outputs,
                    parameter_values,
                };
                Ok(Some(prepared_template))
            }
            None => Ok(None),
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

    fn collect_all_content(from: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let walker = WalkDir::new(&from);
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
    fn read_all(paths: Vec<PathBuf>) -> anyhow::Result<Vec<(PathBuf, TemplateContent)>> {
        let template_parser = Self::template_parser();
        let contents = paths
            .iter()
            .map(std::fs::read)
            .map(|c| c.map(|cc| TemplateContent::infer_from_bytes(cc, &template_parser)))
            .collect::<Result<Vec<_>, _>>()?;
        let pairs = paths.into_iter().zip(contents).collect();
        Ok(pairs)
    }

    // TODO: we can unify most of this with populate_parameters_silent
    fn populate_parameters_interactive(&self) -> anyhow::Result<Option<HashMap<String, String>>> {
        let mut values = HashMap::new();
        for parameter in self.template.parameters() {
            match self.populate_parameter_interactive(parameter) {
                Some(v) => {
                    values.insert(parameter.id().to_owned(), v);
                }
                None => return Ok(None),
            }
        }
        Ok(Some(values))
    }

    fn populate_parameter_interactive(&self, parameter: &TemplateParameter) -> Option<String> {
        match self.options.values.get(parameter.id()) {
            Some(s) => Some(s.clone()),
            None => crate::interaction::prompt_parameter(parameter),
        }
    }

    fn populate_parameters_silent(&self) -> anyhow::Result<Option<HashMap<String, String>>> {
        let mut values = HashMap::new();
        for parameter in self.template.parameters() {
            let value = self.populate_parameter_silent(parameter)?;
            values.insert(parameter.id().to_owned(), value);
        }
        Ok(Some(values))
    }

    fn populate_parameter_silent(&self, parameter: &TemplateParameter) -> anyhow::Result<String> {
        match self.options.values.get(parameter.id()) {
            Some(s) => Ok(s.clone()),
            None => Err(anyhow!("Parameter '{}' not provided", parameter.id())),
        }
    }

    fn to_output_paths<T>(
        src_dir: &Path,
        dest_dir: &Path,
        contents: Vec<(PathBuf, T)>,
    ) -> HashMap<PathBuf, T> {
        let outputs_iter = contents
            .into_iter()
            .filter_map(|f| Self::to_output_path(src_dir, dest_dir, f));
        HashMap::from_iter(outputs_iter)
    }

    fn to_output_path<T>(
        src_dir: &Path,
        dest_dir: &Path,
        (source, cont): (PathBuf, T),
    ) -> Option<(PathBuf, T)> {
        pathdiff::diff_paths(source, src_dir).map(|rel| (dest_dir.join(rel), cont))
    }

    fn template_parser() -> liquid::Parser {
        liquid::ParserBuilder::with_stdlib()
            .filter(crate::filters::KebabCaseFilterParser)
            .filter(crate::filters::PascalCaseFilterParser)
            .filter(crate::filters::SnakeCaseFilterParser)
            .build()
            .expect("can't fail due to no partials support")
    }
}

impl TemplatePreparationResult {
    /// Writes out the artefacts generated by successful template execution,
    /// or reports an execution error.
    pub async fn execute(self) -> anyhow::Result<()> {
        match self.render().await {
            Cancellable::Err(e) => Err(e),
            Cancellable::Cancelled => Ok(()),
            Cancellable::Ok(outputs) => outputs.write().await,
        }
    }

    async fn render(self) -> Cancellable<TemplateOutputs, anyhow::Error> {
        match self.inner {
            Cancellable::Err(e) => Cancellable::Err(e),
            Cancellable::Cancelled => Cancellable::Cancelled,
            Cancellable::Ok(prepared_template) => match prepared_template.render_all().await {
                Ok(rendered) => Cancellable::Ok(rendered),
                Err(e) => Cancellable::Err(e),
            },
        }
    }
}

impl PreparedTemplate {
    async fn render_all(self) -> anyhow::Result<TemplateOutputs> {
        let globals = self.renderer_globals().await;
        let rendered = self
            .files
            .into_iter()
            .map(|(path, content)| Self::render_one(path, content, &globals))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let outputs = HashMap::from_iter(rendered);
        Ok(TemplateOutputs { files: outputs })
    }

    fn render_one(
        path: PathBuf,
        content: TemplateContent,
        globals: &liquid::Object,
    ) -> anyhow::Result<(PathBuf, Vec<u8>)> {
        let rendered = content.render(globals)?;
        Ok((path, rendered))
    }

    async fn renderer_globals(&self) -> liquid::Object {
        let mut object = liquid::Object::new();

        let authors = crate::environment::get_authors().await.unwrap_or_default();
        object.insert(
            "authors".into(),
            liquid_core::Value::Scalar(authors.author.into()),
        );
        object.insert(
            "username".into(),
            liquid_core::Value::Scalar(authors.username.into()),
        );

        for (k, v) in &self.parameter_values {
            object.insert(
                k.to_owned().into(),
                liquid_core::Value::Scalar(v.to_owned().into()),
            );
        }

        object
    }
}

impl TemplateContent {
    fn infer_from_bytes(raw: Vec<u8>, parser: &liquid::Parser) -> TemplateContent {
        match string_from_bytes(&raw) {
            None => TemplateContent::Binary(raw),
            Some(s) => {
                match parser.parse(&s) {
                    Ok(t) => TemplateContent::Template(t),
                    Err(_) => TemplateContent::Binary(raw), // TODO: detect legit broken templates and error on them
                }
            }
        }
    }

    fn render(self, globals: &liquid::Object) -> anyhow::Result<Vec<u8>> {
        match self {
            Self::Template(t) => {
                let text = t.render(globals)?;
                Ok(text.bytes().collect())
            }
            Self::Binary(v) => Ok(v),
        }
    }
}

fn string_from_bytes(bytes: &[u8]) -> Option<String> {
    match std::str::from_utf8(bytes) {
        Ok(s) => Some(s.to_owned()),
        Err(_) => None, // TODO: try other encodings!
    }
}

impl TemplateOutputs {
    pub async fn write(&self) -> anyhow::Result<()> {
        for (path, contents) in &self.files {
            let dir = path
                .parent()
                .with_context(|| format!("Can't get directory containing {}", path.display()))?;
            tokio::fs::create_dir_all(&dir)
                .await
                .with_context(|| format!("Failed to create directory {}", dir.display()))?;
            tokio::fs::write(&path, &contents)
                .await
                .with_context(|| format!("Failed to write file {}", path.display()))?;
        }
        Ok(())
    }
}
