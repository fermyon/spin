use anyhow::anyhow;
use std::{collections::HashMap, path::PathBuf};

use crate::writer::{TemplateOutput, TemplateOutputs};

// A template that has been evaluated and parsed, with all the values
// it needs to render.
pub(crate) struct TemplateRenderer {
    pub render_operations: Vec<RenderOperation>,
    pub parameter_values: HashMap<String, String>,
}

pub(crate) enum TemplateContent {
    Template(liquid::Template),
    Binary(Vec<u8>),
}

pub(crate) enum RenderOperation {
    AppendToml(PathBuf, TemplateContent),
    WriteFile(PathBuf, TemplateContent),
}

impl TemplateRenderer {
    pub(crate) fn render(self) -> anyhow::Result<TemplateOutputs> {
        let globals = self.renderer_globals();

        let outputs = self
            .render_operations
            .into_iter()
            .map(|so| so.render(&globals))
            .collect::<anyhow::Result<Vec<_>>>()?;

        if outputs.is_empty() {
            return Err(anyhow!("Nothing to create"));
        }

        Ok(TemplateOutputs::new(outputs))
    }

    fn renderer_globals(&self) -> liquid::Object {
        let mut object = liquid::Object::new();

        for (k, v) in &self.parameter_values {
            object.insert(
                k.to_owned().into(),
                liquid_core::Value::Scalar(v.to_owned().into()),
            );
        }

        object
    }
}

impl RenderOperation {
    fn render(self, globals: &liquid::Object) -> anyhow::Result<TemplateOutput> {
        match self {
            Self::WriteFile(path, content) => {
                let rendered = content.render(globals)?;
                Ok(TemplateOutput::WriteFile(path, rendered))
            }
            Self::AppendToml(path, content) => {
                let rendered = content.render(globals)?;
                let rendered_text = String::from_utf8(rendered)?;
                Ok(TemplateOutput::AppendToml(path, rendered_text))
            }
        }
    }
}

impl TemplateContent {
    pub(crate) fn infer_from_bytes(raw: Vec<u8>, parser: &liquid::Parser) -> TemplateContent {
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

// TODO: this doesn't truly belong in a module that claims to be about
// rendering but the only thing that uses it is the TemplateContent ctor
fn string_from_bytes(bytes: &[u8]) -> Option<String> {
    match std::str::from_utf8(bytes) {
        Ok(s) => Some(s.to_owned()),
        Err(_) => None, // TODO: try other encodings!
    }
}
