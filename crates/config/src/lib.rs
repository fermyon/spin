pub mod host_component;
pub mod provider;

mod template;
mod tree;

use std::fmt::Debug;

pub use provider::Provider;
pub use tree::{Path, Tree};

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
    #[error("unknown config path {0}")]
    UnknownPath(String),
}

type Result<T> = std::result::Result<T, Error>;

/// A configuration resolver.
#[derive(Debug, Default)]
pub struct Resolver {
    tree: Tree,
    providers: Vec<Box<dyn Provider>>,
}

impl Resolver {
    /// Creates a Resolver for the given Tree.
    pub fn new(tree: Tree) -> Result<Self> {
        Ok(Self {
            tree,
            providers: vec![],
        })
    }

    /// Adds a config Provider to the Resolver.
    pub fn add_provider(&mut self, provider: impl Provider + 'static) {
        self.providers.push(Box::new(provider));
    }

    /// Resolves a config value for the given path.
    pub fn resolve(&self, path: &Path) -> Result<String> {
        self.resolve_path(path, 0)
    }

    // Simple protection against infinite recursion
    const RECURSION_LIMIT: usize = 100;

    // TODO(lann): make this non-recursive and/or "flatten" templates
    fn resolve_path(&self, path: &Path, depth: usize) -> Result<String> {
        let depth = depth + 1;
        if depth > Self::RECURSION_LIMIT {
            return Err(Error::InvalidTemplate(format!(
                "hit recursion limit at path {:?}",
                path
            )));
        }
        let slot = self.tree.get(path)?;
        // If we're resolving top-level config we are ready to query provider(s).
        if path.size() == 1 {
            let key = path.keys().next().unwrap();
            for provider in &self.providers {
                if let Some(value) = provider.get(&key).map_err(Error::Provider)? {
                    return Ok(value);
                }
            }
        }
        // Resolve default template
        if let Some(template) = &slot.default {
            self.resolve_template(path, template, depth)
        } else {
            Err(Error::InvalidPath(format!(
                "missing value at required path {:?}",
                path
            )))
        }
    }

    fn resolve_template(&self, path: &Path, template: &Template, depth: usize) -> Result<String> {
        template.parts().try_fold(String::new(), |value, part| {
            Ok(match part {
                Part::Lit(lit) => value + lit,
                Part::Expr(expr) => {
                    let expr_path = if expr.starts_with('.') {
                        path.resolve_relative(expr)?
                    } else {
                        Path::new(expr.to_string())?
                    };
                    value + &self.resolve_path(&expr_path, depth)?
                }
            })
        })
    }
}

/// A config key.
#[derive(Debug, PartialEq)]
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
    use std::collections::HashMap;

    use toml::toml;

    use super::*;

    #[test]
    fn resolver_resolve_defaults() {
        let mut tree: Tree = toml! {
            top_level = { default = "top" }
            top_ref = { default = "{{ top_level }}+{{ top_level }}" }
            top_required = { required = true }
        }
        .try_into()
        .unwrap();
        tree.merge_defaults(
            &Path::new("child").unwrap(),
            toml! {
                subtree_key = "sub"
                top_ref = "{{ top_level }}"
                recurse_ref = "{{ top_ref }}"
                own_ref = "{{ .subtree_key }}"
            }
            .try_into::<HashMap<String, String>>()
            .unwrap(),
        )
        .unwrap();
        tree.merge_defaults(
            &Path::new("child.grandchild").unwrap(),
            toml! {
                top_ref = "{{ top_level }}"
                parent_ref = "{{ ..subtree_key }}"
                mixed_ref = "{{ top_level }}/{{ ..recurse_ref }}"
            }
            .try_into::<HashMap<String, String>>()
            .unwrap(),
        )
        .unwrap();

        let resolver = Resolver::new(tree).unwrap();
        for (path, expected) in [
            ("top_level", "top"),
            ("top_ref", "top+top"),
            ("child.subtree_key", "sub"),
            ("child.top_ref", "top"),
            ("child.recurse_ref", "top+top"),
            ("child.own_ref", "sub"),
            ("child.grandchild.top_ref", "top"),
            ("child.grandchild.parent_ref", "sub"),
            ("child.grandchild.mixed_ref", "top/top+top"),
        ] {
            let path = Path::new(path).unwrap();
            let value = resolver.resolve(&path).unwrap();
            assert_eq!(value, expected, "mismatch at {:?}", path);
        }
    }

    #[test]
    fn resolver_recursion_limit() {
        let resolver = Resolver::new(
            toml! {
                x = { default = "{{y}}" }
                y = { default = "{{x}}" }
            }
            .try_into()
            .unwrap(),
        )
        .unwrap();
        let path = "x".to_string().try_into().unwrap();
        assert!(matches!(
            resolver.resolve(&path),
            Err(Error::InvalidTemplate(_))
        ));
    }

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
