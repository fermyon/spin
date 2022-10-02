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
    pub name: String,
    /// The path at which to generate artefacts.
    pub output_path: PathBuf,
    /// The values to use for template parameters.
    pub values: HashMap<String, String>,
    /// If true accept default values where available
    pub accept_defaults: bool,
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
    special_values: HashMap<String, String>,
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
    pub async fn interactive(&self) -> TemplatePreparationResult {
        let raw_prepared = self
            .run_inner(
                |path| self.check_allow_generate_interactive(path),
                || self.populate_parameters_interactive(),
            )
            .await;
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
    pub async fn silent(&self) -> TemplatePreparationResult {
        let raw_prepared = self
            .run_inner(
                |path| self.check_allow_generate_silent(path),
                || self.populate_parameters_silent(),
            )
            .await;
        let inner = Cancellable::from_result_option(raw_prepared);
        TemplatePreparationResult { inner }
    }

    async fn run_inner(
        &self,
        allow_generate: impl Fn(&Path) -> Cancellable<(), anyhow::Error>,
        populate_parameters: impl Fn() -> anyhow::Result<Option<HashMap<String, String>>>,
    ) -> anyhow::Result<Option<PreparedTemplate>> {
        // TODO: rationalise `path` and `dir`
        let to = self.target_dir();

        match allow_generate(to) {
            Cancellable::Cancelled => return Ok(None),
            Cancellable::Ok(_) => (),
            Cancellable::Err(e) => return Err(e),
        };

        // TODO: Ok(None) means the run was cancelled - this is hard to follow but plays
        // nicely with the Rust ? operator - is there a better way?

        self.validate_provided_values()?;

        let outputs = match self.template.content_dir() {
            None => HashMap::new(),
            Some(path) => {
                let from = path
                    .absolutize()
                    .context("Failed to get absolute path of template directory")?
                    .into_owned();
                let template_content_files = Self::collect_all_content(&from)?;
                // TODO: okay we do want to do *some* parsing here because we don't want
                // to prompt if the template bodies are garbage
                let template_contents = self.read_all(template_content_files)?;
                Self::to_output_paths(&from, to, template_contents)
            }
        };

        match populate_parameters()? {
            Some(parameter_values) => {
                // let outputs = Self::render_all(output_templates, &parameter_values)?;
                let prepared_template = PreparedTemplate {
                    files: outputs,
                    special_values: self.special_values().await,
                    parameter_values,
                };
                Ok(Some(prepared_template))
            }
            None => Ok(None),
        }
    }

    async fn special_values(&self) -> HashMap<String, String> {
        let mut values = HashMap::new();

        let authors = crate::environment::get_authors().await.unwrap_or_default();
        values.insert("authors".into(), authors.author);
        values.insert("username".into(), authors.username);
        values.insert("project-name".into(), self.options.name.clone());

        values
    }

    fn target_dir(&self) -> &PathBuf {
        &self.options.output_path
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
    fn read_all(&self, paths: Vec<PathBuf>) -> anyhow::Result<Vec<(PathBuf, TemplateContent)>> {
        let template_parser = self.template_parser();
        let contents = paths
            .iter()
            .map(std::fs::read)
            .map(|c| c.map(|cc| TemplateContent::infer_from_bytes(cc, &template_parser)))
            .collect::<Result<Vec<_>, _>>()?;
        let pairs = paths.into_iter().zip(contents).collect();
        Ok(pairs)
    }

    fn check_allow_generate_interactive(
        &self,
        target_dir: &Path,
    ) -> Cancellable<(), anyhow::Error> {
        if !is_directory_empty(target_dir) {
            let prompt = format!(
                "{} already contains other files. Generate into it anyway?",
                target_dir.display()
            );
            match crate::interaction::confirm(&prompt) {
                Ok(true) => Cancellable::Ok(()),
                Ok(false) => Cancellable::Cancelled,
                Err(e) => Cancellable::Err(anyhow::Error::from(e)),
            }
        } else {
            Cancellable::Ok(())
        }
    }

    fn check_allow_generate_silent(&self, target_dir: &Path) -> Cancellable<(), anyhow::Error> {
        if is_directory_empty(target_dir) {
            Cancellable::Ok(())
        } else {
            let err = anyhow!(
                "Can't generate into {} as it already contains other files",
                target_dir.display()
            );
            Cancellable::Err(err)
        }
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
            None => match (self.options.accept_defaults, parameter.default_value()) {
                (true, Some(v)) => Some(v.to_string()),
                _ => crate::interaction::prompt_parameter(parameter),
            },
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
            None => match (self.options.accept_defaults, parameter.default_value()) {
                (true, Some(v)) => Ok(v.to_string()),
                _ => Err(anyhow!("Parameter '{}' not provided", parameter.id())),
            },
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

    fn template_parser(&self) -> liquid::Parser {
        let mut builder = liquid::ParserBuilder::with_stdlib()
            .filter(crate::filters::KebabCaseFilterParser)
            .filter(crate::filters::PascalCaseFilterParser)
            .filter(crate::filters::SnakeCaseFilterParser);
        for filter in self.template.custom_filters() {
            builder = builder.filter(filter);
        }
        builder
            .build()
            .expect("can't fail due to no partials support")
    }
}

fn is_directory_empty(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    if !path.is_dir() {
        return false;
    }
    match path.read_dir() {
        Err(_) => false,
        Ok(mut read_dir) => read_dir.next().is_none(),
    }
}

impl TemplatePreparationResult {
    /// Writes out the artefacts generated by successful template execution,
    /// or reports an execution error.
    pub async fn execute(self) -> anyhow::Result<()> {
        match self.render() {
            Cancellable::Err(e) => Err(e),
            Cancellable::Cancelled => Ok(()),
            Cancellable::Ok(outputs) => outputs.write().await,
        }
    }

    fn render(self) -> Cancellable<TemplateOutputs, anyhow::Error> {
        match self.inner {
            Cancellable::Err(e) => Cancellable::Err(e),
            Cancellable::Cancelled => Cancellable::Cancelled,
            Cancellable::Ok(prepared_template) => match prepared_template.render_all() {
                Ok(rendered) => Cancellable::Ok(rendered),
                Err(e) => Cancellable::Err(e),
            },
        }
    }
}

impl PreparedTemplate {
    fn render_all(self) -> anyhow::Result<TemplateOutputs> {
        let globals = self.renderer_globals();
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

    fn renderer_globals(&self) -> liquid::Object {
        let mut object = liquid::Object::new();

        for (k, v) in &self.special_values {
            object.insert(
                k.to_owned().into(),
                liquid_core::Value::Scalar(v.to_owned().into()),
            );
        }

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
