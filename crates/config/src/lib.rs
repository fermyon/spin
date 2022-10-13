mod host_component;
pub mod provider;
mod template;

use std::{borrow::Cow, collections::HashMap, fmt::Debug};

use spin_app::Variable;

pub use crate::{host_component::ConfigHostComponent, provider::Provider};
use template::{Part, Template};

/// A configuration resolver.
#[derive(Debug, Default)]
pub struct Resolver {
    // variable key -> variable
    variables: HashMap<String, Variable>,
    // component ID -> config key -> config value template
    component_configs: HashMap<String, HashMap<String, Template>>,
    providers: Vec<Box<dyn Provider>>,
}

impl Resolver {
    /// Creates a Resolver for the given Tree.
    pub fn new(variables: impl IntoIterator<Item = (String, Variable)>) -> Result<Self> {
        let variables: HashMap<_, _> = variables.into_iter().collect();
        // Validate keys so that we can rely on them during resolution
        variables.keys().try_for_each(|key| Key::validate(key))?;
        Ok(Self {
            variables,
            component_configs: Default::default(),
            providers: Default::default(),
        })
    }

    /// Adds component configuration values to the Resolver.
    pub fn add_component_config(
        &mut self,
        component_id: impl Into<String>,
        config: impl IntoIterator<Item = (String, String)>,
    ) -> Result<()> {
        let component_id = component_id.into();
        let templates = config
            .into_iter()
            .map(|(key, val)| {
                // Validate config keys so that we can rely on them during resolution
                Key::validate(&key)?;
                let template = self.validate_template(val)?;
                Ok((key, template))
            })
            .collect::<Result<_>>()?;

        self.component_configs.insert(component_id, templates);

        Ok(())
    }

    /// Adds a config Provider to the Resolver.
    pub fn add_provider(&mut self, provider: Box<dyn Provider>) {
        self.providers.push(provider);
    }

    /// Resolves a config value for the given path.
    pub async fn resolve(&self, component_id: &str, key: Key<'_>) -> Result<String> {
        let configs = self.component_configs.get(component_id).ok_or_else(|| {
            Error::UnknownPath(format!("no config for component {component_id:?}"))
        })?;

        let key = key.as_ref();
        let template = configs
            .get(key)
            .ok_or_else(|| Error::UnknownPath(format!("no config for {component_id:?}.{key:?}")))?;

        self.resolve_template(template).await
    }

    async fn resolve_template(&self, template: &Template) -> Result<String> {
        let mut resolved_parts: Vec<Cow<str>> = Vec::with_capacity(template.parts().len());
        for part in template.parts() {
            resolved_parts.push(match part {
                Part::Lit(lit) => lit.as_ref().into(),
                Part::Expr(var) => self.resolve_variable(var).await?.into(),
            });
        }
        Ok(resolved_parts.concat())
    }

    async fn resolve_variable(&self, key: &str) -> Result<String> {
        let var = self
            .variables
            .get(key)
            // This should have been caught by validate_template
            .ok_or_else(|| Error::InvalidKey(key.to_string()))?;

        for provider in &self.providers {
            if let Some(value) = provider.get(&Key(key)).await.map_err(Error::Provider)? {
                return Ok(value);
            }
        }

        var.default.clone().ok_or_else(|| {
            Error::Provider(anyhow::anyhow!(
                "no provider resolved required variable {key:?}"
            ))
        })
    }

    fn validate_template(&self, template: String) -> Result<Template> {
        let template = Template::new(template)?;
        // Validate template variables are valid
        template.parts().try_for_each(|part| match part {
            Part::Expr(var) if !self.variables.contains_key(var.as_ref()) => {
                Err(Error::InvalidTemplate(format!("unknown variable {var:?}")))
            }
            _ => Ok(()),
        })?;
        Ok(template)
    }
}

/// A config key
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
        .map_err(|reason| Error::InvalidKey(format!("{key:?} {reason}")))
    }
}

impl<'a> AsRef<str> for Key<'a> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

type Result<T> = std::result::Result<T, Error>;

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
    Provider(#[source] anyhow::Error),

    /// Unknown config path.
    #[error("unknown config path: {0}")]
    UnknownPath(String),
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;

    #[derive(Debug)]
    struct TestProvider;

    #[async_trait]
    impl Provider for TestProvider {
        async fn get(&self, key: &Key) -> anyhow::Result<Option<String>> {
            match key.as_ref() {
                "required" => Ok(Some("provider-value".to_string())),
                "broken" => anyhow::bail!("broken"),
                _ => Ok(None),
            }
        }
    }

    async fn test_resolve(config_template: &str) -> Result<String> {
        let mut resolver = Resolver::new([
            (
                "required".into(),
                Variable {
                    default: None,
                    secret: false,
                },
            ),
            (
                "default".into(),
                Variable {
                    default: Some("default-value".into()),
                    secret: false,
                },
            ),
        ])
        .unwrap();
        resolver
            .add_component_config(
                "test-component",
                [("test_key".into(), config_template.into())],
            )
            .unwrap();
        resolver.add_provider(Box::new(TestProvider));
        resolver.resolve("test-component", Key("test_key")).await
    }

    #[tokio::test]
    async fn resolve_static() {
        assert_eq!(test_resolve("static-value").await.unwrap(), "static-value");
    }

    #[tokio::test]
    async fn resolve_variable_default() {
        assert_eq!(
            test_resolve("prefix-{{ default }}-suffix").await.unwrap(),
            "prefix-default-value-suffix"
        );
    }

    #[tokio::test]
    async fn resolve_variable_provider() {
        assert_eq!(
            test_resolve("prefix-{{ required }}-suffix").await.unwrap(),
            "prefix-provider-value-suffix"
        );
    }

    #[test]
    fn keys_good() {
        for key in ["a", "abc", "a1b2c3", "a_1", "a_1_b_3"] {
            Key::new(key).expect(key);
        }
    }

    #[test]
    fn keys_bad() {
        for key in ["", "aX", "1bc", "_x", "x.y", "x_", "a__b", "x-y"] {
            Key::new(key).expect_err(key);
        }
    }
}
