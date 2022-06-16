use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use path_absolutize::Absolutize;
use tokio::{fs::File, io::AsyncReadExt};

use spin_templates::{RunOptions, TemplateManager};

/// Scaffold a new application or component based on a template.
#[derive(Parser, Debug)]
pub struct NewCommand {
    /// The template from which to create the new application or component. Run `spin templates list` to see available options.
    pub template_id: String,

    /// The name of the new application or component.
    pub name: String,

    /// The directory in which to create the new application or component.
    /// The default is the name argument.
    #[clap(short = 'o', long = "output")]
    pub output_path: Option<PathBuf>,

    /// Parameter values to be passed to the template (in name=value format).
    #[clap(short = 'v', long = "value", multiple_occurrences = true)]
    pub values: Vec<ParameterValue>,

    /// A TOML file which contains parameter values in name = "value" format.
    /// Parameters passed as CLI option overwrite parameters specified in the
    /// file.
    #[clap(long = "values-file")]
    pub values_file: Option<PathBuf>,
}

impl NewCommand {
    pub async fn run(&self) -> Result<()> {
        let template_manager =
            TemplateManager::default().context("Failed to construct template directory path")?;
        let template = template_manager
            .get(&self.template_id)
            .with_context(|| format!("Error retrieving template {}", self.template_id))?;
        let output_path = self
            .output_path
            .clone()
            .unwrap_or_else(|| path_safe(&self.name));
        let values = {
            let mut values = match self.values_file.as_ref() {
                Some(file) => values_from_file(file.as_path()).await?,
                None => HashMap::new(),
            };
            merge_values(&mut values, &self.values);
            values
        };
        let options = RunOptions {
            name: self.name.clone(),
            output_path,
            values,
        };

        match template {
            Some(template) => template.run(options).interactive().await.execute().await,
            None => {
                // TODO: guidance experience
                println!("Template {} not found", self.template_id);
                Ok(())
            }
        }
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

async fn values_from_file(file: impl AsRef<Path>) -> Result<HashMap<String, String>> {
    let file = file
        .as_ref()
        .absolutize()
        .context("Failed to resolve absolute path to values file")?;

    let mut buf = vec![];
    File::open(file.as_ref())
        .await?
        .read_to_end(&mut buf)
        .await
        .with_context(|| anyhow!("Cannot read values file from {:?}", file.as_ref()))?;

    toml::from_slice(&buf).context("Failed to deserialize values file")
}

/// Merges values from file and values passed as command line options. CLI
/// options take precedence by overwriting values defined in the file.
fn merge_values(from_file: &mut HashMap<String, String>, from_cli: &[ParameterValue]) {
    for value in from_cli {
        from_file.insert(value.name.to_owned(), value.value.to_owned());
    }
}

lazy_static::lazy_static! {
    static ref PATH_UNSAFE_CHARACTERS: regex::Regex = regex::Regex::new("[^-_.a-zA-Z0-9]").expect("Invalid path safety regex");
}

fn path_safe(text: &str) -> PathBuf {
    let path = PATH_UNSAFE_CHARACTERS.replace_all(text, "_");
    PathBuf::from(path.to_string())
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
}
