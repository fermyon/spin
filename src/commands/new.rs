use anyhow::{anyhow, Context, Result};
use spin_templates::{RunOptions, TemplateManager};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use structopt::StructOpt;

/// Scaffold a new application or component based on a template.
#[derive(StructOpt, Debug)]
pub struct NewCommand {
    /// The template from which to create the new application or component.
    pub template_id: String,

    /// The name of the new application or component.
    #[structopt(short = "n", long = "name")]
    pub name: Option<String>,

    /// The directory in which to create the new application or component.
    #[structopt(short = "o", long = "output")]
    pub output_path: Option<PathBuf>,

    /// Parameter values to be passed to the template (in name=value format).
    #[structopt(short = "v", long = "value", multiple = true)]
    pub values: Vec<ParameterValue>,
}

impl NewCommand {
    pub async fn run(&self) -> Result<()> {
        let template_manager =
            TemplateManager::default().context("Failed to construct template directory path")?;
        let template = template_manager
            .get(&self.template_id)
            .with_context(|| format!("Error retrieving template {}", self.template_id))?;
        let options = RunOptions {
            name: self.name.clone(),
            output_path: self.output_path.clone(),
            values: to_hash_map(&self.values),
        };

        match template {
            Some(template) => template.run(options).interactive().execute().await,
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

fn to_hash_map(values: &[ParameterValue]) -> HashMap<String, String> {
    values
        .iter()
        .map(|p| (p.name.clone(), p.value.clone()))
        .collect()
}
