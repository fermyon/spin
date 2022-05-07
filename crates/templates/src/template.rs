use std::path::PathBuf;

use anyhow::{anyhow, Context};
use indexmap::IndexMap;
use regex::Regex;

use crate::{
    constraints::StringConstraints,
    reader::{RawParameter, RawTemplateManifest},
    run::{Run, RunOptions},
    store::TemplateLayout,
};

#[derive(Debug)]
pub struct Template {
    id: String,
    description: Option<String>,
    parameters: Vec<TemplateParameter>,
    content_dir: Option<PathBuf>, // TODO: maybe always need a spin.toml file in there?
}

#[derive(Clone, Debug)]
pub(crate) enum TemplateParameterDataType {
    String(StringConstraints),
}

#[derive(Debug)]
pub(crate) struct TemplateParameter {
    id: String,
    data_type: TemplateParameterDataType, // TODO: possibly abstract to a ValidationCriteria type?
    prompt: String,
    default_value: Option<String>,
}

impl Template {
    pub(crate) fn load_from(layout: &TemplateLayout) -> anyhow::Result<Self> {
        let manifest_path = layout.manifest_path();

        let manifest_text = std::fs::read_to_string(&manifest_path).with_context(|| {
            format!(
                "Failed to read template manifest file {}",
                manifest_path.display()
            )
        })?;
        let raw = crate::reader::parse_manifest_toml(&manifest_text).with_context(|| {
            format!(
                "Manifest file {} is not a valid manifest",
                manifest_path.display()
            )
        })?;

        let content_dir = if layout.content_dir().exists() {
            Some(layout.content_dir())
        } else {
            None
        };

        let template = match raw {
            RawTemplateManifest::V1(raw) => Self {
                id: raw.id.clone(),
                description: raw.description.clone(),
                parameters: Self::parse_parameters(&raw.parameters)?,
                content_dir,
            },
        };
        Ok(template)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn description(&self) -> &Option<String> {
        &self.description
    }

    pub fn description_or_empty(&self) -> &str {
        match &self.description {
            Some(s) => s,
            None => "",
        }
    }

    pub(crate) fn parameters(&self) -> impl Iterator<Item = &TemplateParameter> {
        self.parameters.iter()
    }

    pub(crate) fn parameter(&self, name: impl AsRef<str>) -> Option<&TemplateParameter> {
        self.parameters.iter().find(|p| p.id == name.as_ref())
    }

    pub(crate) fn content_dir(&self) -> &Option<PathBuf> {
        &self.content_dir
    }

    pub fn run(self, options: RunOptions) -> Run {
        Run::new(self, options)
    }

    fn parse_parameters(
        raw: &Option<IndexMap<String, RawParameter>>,
    ) -> anyhow::Result<Vec<TemplateParameter>> {
        match raw {
            None => Ok(vec![]),
            Some(parameters) => parameters
                .iter()
                .map(|(k, v)| TemplateParameter::from_raw(k, v))
                .collect(),
        }
    }
}

impl TemplateParameter {
    fn from_raw(id: &str, raw: &RawParameter) -> anyhow::Result<Self> {
        let data_type = TemplateParameterDataType::parse(raw)?;

        Ok(Self {
            id: id.to_owned(),
            data_type,
            prompt: raw.prompt.clone(),
            default_value: raw.default_value.clone(),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn data_type(&self) -> &TemplateParameterDataType {
        &self.data_type
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn default_value(&self) -> &Option<String> {
        &self.default_value
    }

    pub fn validate_value(&self, value: impl AsRef<str>) -> anyhow::Result<String> {
        self.data_type.validate_value(value.as_ref().to_owned())
    }
}

impl TemplateParameterDataType {
    fn parse(raw: &RawParameter) -> anyhow::Result<Self> {
        match &raw.data_type[..] {
            "string" => Ok(Self::String(parse_string_constraints(raw)?)),
            _ => Err(anyhow!("Unrecognised data type '{}'", raw.data_type)),
        }
    }

    fn validate_value(&self, value: String) -> anyhow::Result<String> {
        match self {
            TemplateParameterDataType::String(constraints) => constraints.validate(value),
        }
    }
}

fn parse_string_constraints(raw: &RawParameter) -> anyhow::Result<StringConstraints> {
    let regex = raw.pattern.as_ref().map(|re| Regex::new(re)).transpose()?;

    Ok(StringConstraints { regex })
}
