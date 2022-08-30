mod host_component;
pub mod provider;

mod template;

use std::{borrow::Cow, collections::HashMap, fmt::Debug};

use anyhow::{anyhow, Context};
use spin_app::{App, Variable};

pub use host_component::ConfigHostComponent;
pub use provider::Provider;
use template::{Part, Template};

/// A config resolution error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid config key.
    #[error("invalid config key: {0}")]
    InvalidKey(String),

    /// Invalid config path.
    #[error("invalid config path: {0}")]
    InvalidPath(String),

    /// Invalid config schema.
    #[error("invalid config schema: {0}")]
    InvalidSchema(String),

    /// Invalid config template.
    #[error("invalid config template: {0}")]
    InvalidTemplate(String),

    /// Config provider error.
    #[error("provider error: {0:?}")]
    Provider(anyhow::Error),

    /// Unknown config path.
    #[error("unknown config path: {0}")]
    UnknownPath(String),
}

type Result<T> = std::result::Result<T, Error>;

/// A configuration resolver.
#[derive(Debug, Default)]
pub struct Resolver {
    variables: HashMap<String, Variable>,
    // component_id -> key -> template
    component_configs: HashMap<String, HashMap<String, Template>>,
    providers: Vec<Box<dyn Provider>>,
}

impl Resolver {
    /// Creates a Resolver for the given Tree.
    pub fn new(app: &App) -> anyhow::Result<Self> {
        let variables = app
            .variables()
            .map(|(key, var)| (key.clone(), var.clone()))
            .collect();

        let mut component_configs = HashMap::new();
        for component in app.components() {
            let templates: &mut HashMap<_, _> = component_configs
                .entry(component.id().to_string())
                .or_default();
            for (key, val) in component.config() {
                Key::validate(key).with_context(|| {
                    format!(
                        "invalid config key {:?} for component {:?}",
                        key,
                        component.id()
                    )
                })?;
                let template = Template::new(val.as_str()).with_context(|| {
                    format!(
                        "invalid config value for {:?} for component {:?}",
                        key,
                        component.id()
                    )
                })?;
                templates.insert(key.clone(), template);
            }
        }

        Ok(Self {
            variables,
            component_configs,
            providers: Default::default(),
        })
    }

    /// Adds a config Provider to the Resolver.
    pub fn add_provider(&mut self, provider: impl Into<Box<dyn Provider>>) {
        self.providers.push(provider.into());
    }

    /// Resolve a config value for the given component and key.
    pub async fn resolve(&self, component_id: &str, key: Key<'_>) -> Result<String> {
        let component_config = self.component_configs.get(component_id).ok_or_else(|| {
            Error::UnknownPath(format!("no config for component {component_id:?}"))
        })?;

        let template = component_config
            .get(key.as_ref())
            .ok_or_else(|| Error::UnknownPath(format!("no config for {component_id:?}.{key:?}")))?;

        self.resolve_template(template).await.map_err(|err| {
            Error::InvalidTemplate(format!(
                "failed to resolve template for {component_id:?}.{key:?}: {err:?}"
            ))
        })
    }

    async fn resolve_template(&self, template: &Template) -> Result<String> {
        let mut resolved: Vec<Cow<str>> = Vec::with_capacity(template.parts().len());
        for part in template.parts() {
            resolved.push(match part {
                Part::Lit(lit) => lit.as_ref().into(),
                Part::Expr(expr) => self.resolve_expr(expr).await?,
            });
        }
        Ok(resolved.concat())
    }

    async fn resolve_expr(&self, expr: &str) -> Result<Cow<str>> {
        let var = self
            .variables
            .get(expr)
            .ok_or_else(|| Error::UnknownPath(format!("no variable named {expr:?}")))?;

        for provider in &self.providers {
            if let Some(value) = provider.get(&Key(expr)).await.map_err(Error::Provider)? {
                return Ok(value.into());
            }
        }

        match var.default {
            Some(ref default) => Ok(default.into()),
            None => Err(Error::Provider(anyhow!(
                "no provider resolved variable {expr:?}"
            ))),
        }
    }
}

/// A config key.
#[derive(Debug, PartialEq, Eq)]
pub struct Key<'a>(&'a str);

impl<'a> Key<'a> {
    /// Creates a new Key.
    pub fn new(key: &'a str) -> Result<Self> {
        Self::validate(key)?;
        Ok(Self(key))
    }

    // To allow various (env var, file path) transformations:
    // - must start with an ASCII letter
    // - underscores are allowed; one at a time between other characters
    // - all other characters must be ASCII alphanumeric
    fn validate(key: &str) -> Result<()> {
        {
            if key.is_empty() {
                Err("may not be empty".to_string())
            } else if !key.bytes().next().unwrap().is_ascii_lowercase() {
                Err("must start with an ASCII letter".to_string())
            } else if !key.bytes().last().unwrap().is_ascii_alphanumeric() {
                Err("must end with an ASCII alphanumeric char".to_string())
            } else if key.contains("__") {
                Err("may not contain multiple consecutive underscores".to_string())
            } else if let Some(invalid) = key
                .chars()
                .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == &'_'))
            {
                Err(format!("invalid character {:?}", invalid))
            } else {
                Ok(())
            }
        }
        .map_err(Error::InvalidKey)
    }
}

impl<'a> AsRef<str> for Key<'a> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_good() {
        for key in ["a", "abc", "a1b2c3", "a_1", "a_1_b_3"] {
            Key::new(key).expect(key);
        }
    }

    #[test]
    fn keys_bad() {
        for key in ["", "aX", "1bc", "_x", "x_", "a__b", "x-y"] {
            Key::new(key).expect_err(key);
        }
    }
}
