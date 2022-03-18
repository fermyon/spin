use crate::emoji;
use anyhow::{bail, Result};
use console::style;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use structopt::StructOpt;

const TEMPLATE_ID_OPT: &str = "TEMPLATE_ID";

/// Arguments for generating templates.
#[derive(Debug, StructOpt)]
pub struct TemplateArgs {
    /// Pass template values through a file. Values should be in the format `key=value`, one per line
    #[structopt(long)]
    pub values_file: Option<PathBuf>,
    /// Define a value for use during template expansion
    #[structopt(long = "define", short = "d", number_of_values = 1)]
    pub value_defs: Vec<String>,
    /// The destination where the template will be generated.
    #[structopt(short, long)]
    pub output: PathBuf,
    /// Use the local repository to resolve the template id.
    #[structopt(long)]
    pub local: bool,
    /// If silent mode is set all variables will be extracted from the values file.
    /// If a value is missing the project generation will fail
    #[structopt(long)]
    pub noprompt: bool,
    /// Enable verbose output.
    #[structopt(long)]
    pub verbose: bool,
    /// The template to use for generating a project.
    #[structopt(name = TEMPLATE_ID_OPT)]
    pub template_id: TemplateId,
}

/// The template id in `repo:name` form.
#[derive(Clone, Debug)]
pub struct TemplateId {
    /// Name of template repository
    pub repo: String,
    /// Name of template
    pub name: String, // TODO: should we formalize the validity of these names?
}

impl FromStr for TemplateId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            bail!("Template ID must be of the form `repo:name`");
        }
        let repo = parts[0].to_owned();
        let name = parts[1].to_owned();

        Ok(TemplateId { repo, name })
    }
}

impl TemplateArgs {
    pub(crate) fn resolve_values(
        &self,
        defaults: Option<HashMap<String, toml::Value>>,
    ) -> Result<HashMap<String, toml::Value>> {
        resolve_template_values(defaults, &self.value_defs, &self.values_file)
    }
}

fn resolve_template_values(
    defaults: Option<HashMap<String, toml::Value>>,
    defines: &[String],
    values_file: &Option<PathBuf>,
) -> Result<HashMap<String, toml::Value>> {
    let mut values = defaults.unwrap_or_default();

    values.extend(
        env::var("SPIN_TEMPLATE_VALUES_FILE")
            .ok()
            .map_or(Ok(Default::default()), |path| {
                read_template_values_file(Path::new(&path))
            })?,
    );

    values.extend(env::vars().filter_map(|(key, value)| {
        key.strip_prefix("SPIN_TEMPLATE_VALUE_")
            .map(|key| (key.to_lowercase(), toml::Value::from(value)))
    }));

    values.extend(
        values_file
            .as_ref()
            .map_or(Ok(Default::default()), |path| {
                read_template_values_file(&path)
            })?,
    );

    add_cli_defined_values(&mut values, defines)?;

    Ok(values)
}

fn read_template_values_file(path: impl AsRef<Path>) -> Result<HashMap<String, toml::Value>> {
    match std::fs::read_to_string(path) {
        Ok(ref contents) => {
            toml::from_str::<HashMap<String, toml::Value>>(contents).map_err(|e| e.into())
        }
        Err(e) => anyhow::bail!(
            "{} {} {}",
            emoji::ERROR,
            style("Values File Error:").bold().red(),
            style(e).bold().red(),
        ),
    }
}

fn add_cli_defined_values<S: AsRef<str> + std::fmt::Display>(
    template_values: &mut HashMap<String, toml::Value>,
    definitions: &[S],
) -> Result<()> {
    let key_value_regex = regex::Regex::new(r"^([a-zA-Z]+[a-zA-Z0-9\-_]*)\s*=\s*(.+)$").unwrap();

    definitions
        .iter()
        .try_fold(
            template_values,
            |template_values, definition| match key_value_regex.captures(definition.as_ref()) {
                Some(cap) => {
                    let key = cap.get(1).unwrap().as_str().to_string();
                    let value = cap.get(2).unwrap().as_str().to_string();
                    println!("{} => '{}'", key, value);
                    template_values.insert(key, toml::Value::from(value));
                    Ok(template_values)
                }
                None => Err(anyhow::anyhow!(
                    "{} {} {}",
                    emoji::ERROR,
                    style("Failed to parse value:").bold().red(),
                    style(definition).bold().red(),
                )),
            },
        )?;
    Ok(())
}
