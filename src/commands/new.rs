use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use path_absolutize::Absolutize;
use tokio;

use spin_loader::local::absolutize;
use spin_templates::{RunOptions, Template, TemplateManager, TemplateVariantInfo};

use crate::opts::{APP_MANIFEST_FILE_OPT, DEFAULT_MANIFEST_FILE};

/// Scaffold a new application based on a template.
#[derive(Parser, Debug)]
pub struct TemplateNewCommandCore {
    /// The name of the new application or component.
    #[clap(value_parser = validate_name)]
    pub name: Option<String>,

    /// The template from which to create the new application or component. Run `spin templates list` to see available options.
    #[clap(short = 't', long = "template")]
    pub template_id: Option<String>,

    /// Filter templates to select by tags.
    #[clap(
        long = "tag",
        multiple_occurrences = true,
        conflicts_with = "template-id"
    )]
    pub tags: Vec<String>,

    /// The directory in which to create the new application or component.
    /// The default is the name argument.
    #[clap(short = 'o', long = "output", group = "location")]
    pub output_path: Option<PathBuf>,

    /// Create the new application or component in the current directory.
    #[clap(long = "init", takes_value = false, group = "location")]
    pub init: bool,

    /// Parameter values to be passed to the template (in name=value format).
    #[clap(short = 'v', long = "value", multiple_occurrences = true)]
    pub values: Vec<ParameterValue>,

    /// A TOML file which contains parameter values in name = "value" format.
    /// Parameters passed as CLI option overwrite parameters specified in the
    /// file.
    #[clap(long = "values-file")]
    pub values_file: Option<PathBuf>,

    /// An optional argument that allows to skip prompts for the manifest file
    /// by accepting the defaults if available on the template
    #[clap(short = 'a', long = "accept-defaults", takes_value = false)]
    pub accept_defaults: bool,
}

/// Scaffold a new application based on a template.
#[derive(Parser, Debug)]
pub struct NewCommand {
    #[clap(flatten)]
    options: TemplateNewCommandCore,
}

/// Scaffold a new component into an existing application.
#[derive(Parser, Debug)]
pub struct AddCommand {
    #[clap(flatten)]
    options: TemplateNewCommandCore,

    /// Path to spin.toml.
    #[clap(
        name = APP_MANIFEST_FILE_OPT,
        short = 'f',
        long = "file",
    )]
    pub app: Option<PathBuf>,
}

impl NewCommand {
    pub async fn run(&self) -> Result<()> {
        self.options.run(TemplateVariantInfo::NewApplication).await
    }
}

impl AddCommand {
    pub async fn run(&self) -> Result<()> {
        let app_file = self
            .app
            .as_deref()
            .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
        let manifest_path = app_file
            .absolutize()
            .with_context(|| {
                format!(
                    "Can't get absolute path for manifest file '{}'",
                    app_file.display()
                )
            })?
            .into_owned();
        if !manifest_path.exists() {
            anyhow::bail!(
                "Can't add component to {}: file does not exist",
                manifest_path.display()
            );
        }
        self.options
            .run(TemplateVariantInfo::AddComponent { manifest_path })
            .await
    }
}

impl TemplateNewCommandCore {
    pub async fn run(&self, variant: TemplateVariantInfo) -> Result<()> {
        let template_manager = TemplateManager::try_default()
            .context("Failed to construct template directory path")?;

        // If a user types `spin new http-rust` etc. then it's *probably* Spin 1.x muscle memory;
        // try to be helpful without getting in the way.
        if let Some(name) = &self.name {
            if self.template_id.is_none() && matches!(template_manager.get(name), Ok(Some(_))) {
                terminal::einfo!(
                    "This will create an app called {name}.",
                    "If you meant to use the {name} template, write '-t {name}'."
                )
            }
        }

        let template = match &self.template_id {
            Some(template_id) => match template_manager
                .get(template_id)
                .with_context(|| format!("Error retrieving template {}", template_id))?
            {
                Some(template) => template,
                None => match prompt_template(&template_manager, &variant, &[template_id.clone()])
                    .await?
                {
                    Some(template) => template,
                    None => return Ok(()),
                },
            },
            None => match prompt_template(&template_manager, &variant, &self.tags).await? {
                Some(template) => template,
                None => return Ok(()),
            },
        };

        if !template.supports_variant(&variant) {
            println!(
                "Template {} doesn't support the '{}' operation",
                template.id(),
                variant.description()
            );
            return Ok(());
        }

        let name = match &self.name {
            Some(name) => name.to_owned(),
            None => prompt_name(&variant).await?,
        };

        let output_path = if self.init {
            PathBuf::from(".")
        } else {
            self.output_path.clone().unwrap_or_else(|| path_safe(&name))
        };

        let values = {
            let mut values = match self.values_file.as_ref() {
                Some(file) => values_from_file(file.as_path()).await?,
                None => HashMap::new(),
            };
            merge_values(&mut values, &self.values);
            values
        };
        let options = RunOptions {
            variant,
            name: name.clone(),
            output_path,
            values,
            accept_defaults: self.accept_defaults,
        };

        template.run(options).interactive().await
    }
}

#[derive(Debug)]
pub struct ParameterValue {
    pub name: String,
    pub value: String,
}

impl FromStr for ParameterValue {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((name, value)) = s.split_once('=') {
            Ok(Self {
                name: name.to_owned(),
                value: value.to_owned(),
            })
        } else {
            Err(anyhow!("'{}' should be in the form name=value", s))
        }
    }
}

/// This function reads a file and parses it as TOML, then
/// returns the resulting hashmap of key-value pairs.
async fn values_from_file(path: impl AsRef<Path>) -> Result<HashMap<String, String>> {
    // Get the absolute path of the file we're reading.
    let path = absolutize(path)?;

    // Open the file.
    let text = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read text from values file {}", path.display()))?;

    // Parse the TOML file into a hashmap of values.
    toml::from_str(&text).context("Failed to deserialize values file")
}

/// Merges values from file and values passed as command line options. CLI
/// options take precedence by overwriting values defined in the file.
fn merge_values(from_file: &mut HashMap<String, String>, from_cli: &[ParameterValue]) {
    for value in from_cli {
        from_file.insert(value.name.to_owned(), value.value.to_owned());
    }
}

async fn prompt_template(
    template_manager: &TemplateManager,
    variant: &TemplateVariantInfo,
    tags: &[String],
) -> anyhow::Result<Option<Template>> {
    let mut templates = match list_or_install_templates(template_manager, tags).await? {
        Some(t) => t,
        None => return Ok(None),
    };
    if templates.is_empty() {
        if tags.len() == 1 {
            bail!("No templates matched '{}'", tags[0]);
        } else {
            bail!("No templates matched all tags");
        }
    }

    let opts = templates
        .iter()
        .map(|t| format!("{} ({})", t.id(), t.description_or_empty()))
        .collect::<Vec<_>>();
    let noun = variant.prompt_noun();
    let prompt = format!("Pick a template to start your {noun} with");
    let index = match dialoguer::Select::new()
        .with_prompt(prompt)
        .items(&opts)
        .default(0)
        .interact_opt()?
    {
        Some(i) => i,
        None => return Ok(None),
    };
    let choice = templates.swap_remove(index);
    Ok(Some(choice))
}

async fn list_or_install_templates(
    template_manager: &TemplateManager,
    tags: &[String],
) -> anyhow::Result<Option<Vec<Template>>> {
    let list_results = template_manager.list_with_tags(tags).await?;
    if list_results.needs_install() {
        super::templates::prompt_install_default_templates(template_manager).await
    } else {
        Ok(Some(list_results.templates))
    }
}

async fn prompt_name(variant: &TemplateVariantInfo) -> anyhow::Result<String> {
    let noun = variant.prompt_noun();
    let mut prompt = format!("Enter a name for your new {noun}");
    loop {
        let result = dialoguer::Input::<String>::new()
            .with_prompt(prompt)
            .interact_text()?;
        if result.trim().is_empty() {
            prompt = format!("Name is required. Try another {noun} name (or Crl+C to exit)");
            continue;
        } else {
            return Ok(result);
        }
    }
}

lazy_static::lazy_static! {
    static ref PATH_UNSAFE_CHARACTERS: regex::Regex = regex::Regex::new("[^-_.a-zA-Z0-9]").expect("Invalid path safety regex");
    static ref NAME: regex::Regex = regex::Regex::new("^[a-zA-Z].*").expect("Invalid name regex");
}

fn path_safe(text: &str) -> PathBuf {
    let path = PATH_UNSAFE_CHARACTERS.replace_all(text, "_");
    PathBuf::from(path.to_string())
}

fn validate_name(name: &str) -> Result<String, String> {
    if NAME.is_match(name) {
        Ok(name.to_owned())
    } else {
        Err("Name must start with a letter".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::{NamedTempFile, TempPath};

    use super::*;

    const TOML_PARAMETER_VALUES: &str = r#"
    key_1 = 'value_1'
    key_2 = 'value_2'
    "#;

    /// Writes to a new temporary file, closes it, and returns its path.
    fn create_tempfile(content: &str) -> Result<TempPath> {
        let mut file = NamedTempFile::new()?;
        write!(file, "{}", content).unwrap();
        Ok(file.into_temp_path())
    }

    #[tokio::test]
    async fn test_values_from_file_empty() {
        let file = create_tempfile("").unwrap();
        let values = values_from_file(&file).await.unwrap();
        assert_eq!(HashMap::new(), values);
    }

    #[tokio::test]
    async fn test_values_from_file_good() {
        let file = create_tempfile(TOML_PARAMETER_VALUES).unwrap();
        let values = values_from_file(&file).await.unwrap();
        let want: HashMap<_, _> = HashMap::from_iter([
            ("key_1".to_owned(), "value_1".to_owned()),
            ("key_2".to_owned(), "value_2".to_owned()),
        ]);
        assert_eq!(want, values);
    }

    #[tokio::test]
    async fn test_values_from_file_bad() {
        let bad_content = [
            "key_1 = 1", // value is not a string
        ];
        for content in bad_content {
            let file = create_tempfile(content).unwrap();
            assert!(
                values_from_file(&file).await.is_err(),
                "content: {}",
                content
            );
        }
    }

    /// Verify values passed as CLI option overwrite values set in file.
    #[test]
    fn merge_values_cli_option_precedence() {
        let mut values = HashMap::from_iter([
            ("key_1".to_owned(), "value_1".to_owned()),
            ("key_2".to_owned(), "value_2".to_owned()),
        ]);
        let from_cli = vec![ParameterValue {
            name: "key_2".to_owned(),
            value: "foo".to_owned(),
        }];
        let want = HashMap::from_iter([
            ("key_1".to_owned(), "value_1".to_owned()),
            ("key_2".to_owned(), "foo".to_owned()),
        ]);
        merge_values(&mut values, &from_cli);
        assert_eq!(want, values);
    }

    #[test]
    fn project_names_must_start_with_letter() {
        assert_eq!("hello", validate_name("hello").unwrap());
        assert_eq!("Proj123!.456", validate_name("Proj123!.456").unwrap());
        validate_name("123").unwrap_err();
        validate_name("1hello").unwrap_err();
        validate_name("_foo").unwrap_err();
    }
}
