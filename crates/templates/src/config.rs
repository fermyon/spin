use crate::{
    emoji, template,
    variable::{self, Variables},
    CONFIG_FILE_NAME,
};
use anyhow::{bail, Result};
use console::style;
use indexmap::IndexMap;
use liquid_core::model::map::Entry;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub(crate) type Parameters = IndexMap<String, toml::Value>;

/// Configuration for a template
#[derive(Deserialize, Debug, PartialEq, Default, Clone)]
pub struct TemplateConfig {
    /// Files to include
    pub include: Option<Vec<String>>,
    /// Files to exclude
    pub exclude: Option<Vec<String>>,
    /// Files to ignore
    pub ignore: Option<Vec<String>>,
    /// Templates hooks
    pub hooks: Option<HooksConfig>,
    /// Template parameter definitions
    pub parameters: Parameters,
}

impl TryFrom<String> for TemplateConfig {
    type Error = toml::de::Error;

    fn try_from(contents: String) -> Result<Self, Self::Error> {
        toml::from_str(&contents)
    }
}

impl TemplateConfig {
    /// Pre-hooks
    pub fn pre_hooks(&self) -> Vec<String> {
        self.hooks().pre.unwrap_or_default()
    }

    /// Post-hooks
    pub fn post_hooks(&self) -> Vec<String> {
        self.hooks().post.unwrap_or_default()
    }

    /// Hooks config
    pub fn hooks(&self) -> HooksConfig {
        self.hooks.clone().unwrap_or_default()
    }

    /// Get template config from path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::try_from(fs::read_to_string(path)?)?)
    }

    /// Returns the parameter variables.
    pub(crate) fn variables(&self) -> Result<Variables> {
        Variables::try_from(&self.parameters)
    }

    /// Returns all hook files
    pub(crate) fn all_hooks(&self) -> Vec<String> {
        let mut all = self.pre_hooks();
        all.extend(self.post_hooks());
        all
    }

    pub(crate) fn object(
        &self,
        noprompt: bool,
        overrides: HashMap<String, toml::Value>,
    ) -> Result<liquid::Object> {
        let authors = template::get_authors()?;

        let mut object = liquid::Object::new();
        object.insert(
            "authors".into(),
            liquid_core::Value::Scalar(authors.author.into()),
        );
        object.insert(
            "username".into(),
            liquid_core::Value::Scalar(authors.username.into()),
        );

        object = self.fill(object, |var| {
            let val = overrides.get(&var.var_name).and_then(|v| v.as_str());
            if val.is_none() && noprompt {
                bail!(variable::ConversionError::MissingPlaceholderVariable {
                    var_name: var.var_name.clone()
                })
            }
            var.resolve(val)
        })?;

        // add missing provided values
        overrides.iter().try_for_each(|(k, v)| {
            if object.contains_key(k.as_str()) {
                return Ok(());
            }
            let val = match v {
                toml::Value::String(content) => liquid_core::Value::Scalar(content.clone().into()),
                toml::Value::Boolean(content) => liquid_core::Value::Scalar((*content).into()),
                _ => anyhow::bail!(format!(
                    "{} {}",
                    emoji::ERROR,
                    style("Unsupported value type. Only Strings and Booleans are supported.")
                        .bold()
                        .red(),
                )),
            };
            object.insert(k.clone().into(), val);
            Ok(())
        })?;

        // todo: merge conditionals

        Ok(object)
    }

    fn fill<F>(&self, mut obj: liquid::Object, values: F) -> Result<liquid::Object>
    where
        F: Fn(&variable::Variable) -> Result<liquid_core::Value>,
    {
        let Variables(vars) = self.variables()?;

        for var in vars {
            match obj.entry(&var.var_name) {
                Entry::Occupied(_) => (), // we already have the value from the config file
                Entry::Vacant(entry) => {
                    // we don't have the value from the config and we can ask for it
                    let value = values(&var)?;
                    entry.insert(value);
                }
            }
        }
        Ok(obj)
    }
}

/// Configuration for template hooks
#[derive(Deserialize, Debug, PartialEq, Default, Clone)]
pub struct HooksConfig {
    pub pre: Option<Vec<String>>,
    pub post: Option<Vec<String>>,
}

pub(crate) fn locate_template_configs(dir: &Path) -> Result<Vec<String>> {
    let mut configs = vec![];

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_name() == CONFIG_FILE_NAME {
            let path = entry
                .path()
                .parent()
                .unwrap()
                .strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .to_string();
            configs.push(path)
        }
    }

    Ok(configs)
}

#[cfg(test)]
mod tests {
    use crate::tests::{create_file, PathString};

    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use toml::Value;

    #[test]
    fn locate_configs_can_locate_paths_with_spin_generate() -> anyhow::Result<()> {
        let tmp = tempdir().unwrap();
        create_file(&tmp, "dir1/spin.toml", "")?;
        create_file(&tmp, "dir2/dir2_1/spin.toml", "")?;
        create_file(&tmp, "dir2/dir2_2/spin-generate.toml", "")?;
        create_file(&tmp, "dir3/spin.toml", "")?;
        create_file(&tmp, "dir4/spin-generate.toml", "")?;

        let expected = vec![
            Path::new("dir2").join("dir2_2").to_string(),
            "dir4".to_string(),
        ];
        let result = {
            let mut x = locate_template_configs(tmp.path())?;
            x.sort();
            x
        };
        assert_eq!(expected, result);
        Ok(())
    }

    #[test]
    fn locate_configs_returns_empty_upon_failure() -> anyhow::Result<()> {
        let tmp = tempdir().unwrap();
        create_file(&tmp, "dir1/spin.toml", "")?;
        create_file(&tmp, "dir2/dir2_1/spin.toml", "")?;
        create_file(&tmp, "dir3/spin.toml", "")?;

        let result = locate_template_configs(tmp.path())?;
        assert_eq!(Vec::new() as Vec<String>, result);
        Ok(())
    }

    #[test]
    fn test_deserializes_config() {
        let test_dir = tempdir().unwrap();
        let config_path = test_dir.path().join(CONFIG_FILE_NAME);
        let mut file = File::create(&config_path).unwrap();

        file.write_all(
            r#"
            include = ["spin.toml"]
            [parameters]
            test = {a = "a"}
        "#
            .as_bytes(),
        )
        .unwrap();

        let config = TemplateConfig::from_path(&config_path).unwrap();

        assert_eq!(config.include, Some(vec!["spin.toml".to_string()]));
        assert!(config.exclude.is_none());
        assert!(config.ignore.is_none());
        assert!(!config.parameters.is_empty());
    }

    #[test]
    fn config_try_from_handles_empty() {
        let result = TemplateConfig::try_from("[parameters]".to_string());
        assert!(result.is_ok(), "Config should have parsed");
        let result = result.unwrap();
        assert_eq!(
            result,
            TemplateConfig {
                include: None,
                exclude: None,
                ignore: None,
                hooks: None,
                parameters: Default::default(),
            }
        )
    }

    #[test]
    fn config_try_from_errors_on_invalid_keys() {
        let result = TemplateConfig::try_from(
            r#"
            [parameters]
            a key = { type = "bool", prompt = "foo"}
            b = { type = "string", prompt = "bar" }
            "#
            .to_string(),
        );

        assert!(result.is_err(), "Config should not have parsed");
    }

    #[test]
    fn config_try_from_handles_parameters() {
        let result = TemplateConfig::try_from(
            r#"
            [parameters]
            a = { type = "bool", prompt = "foo", default = false }
            b = { type = "string", prompt = "bar" }
            "#
            .to_string(),
        );

        assert!(result.is_ok(), "Config should have parsed");
        let result = result.unwrap();

        assert!(
            !result.parameters.is_empty(),
            "parameters should have been filled"
        );
        let parameters = result.parameters;

        assert_eq!(parameters.len(), 2);

        let a = parameters.get("a");
        let b = parameters.get("b");

        assert!(
            a.is_some() && b.is_some(),
            "parameter keys should have been parsed"
        );

        let a_table = a.unwrap().as_table();
        let b_table = b.unwrap().as_table();

        assert!(
            a_table.is_some() && b_table.is_some(),
            "values should have been parsed as tables"
        );

        let a_table = a_table.unwrap();
        let b_table = b_table.unwrap();

        assert_eq!(a_table.len(), 3);
        assert_eq!(b_table.len(), 2);

        let (a_type, a_prompt, a_default) = (
            a_table.get("type"),
            a_table.get("prompt"),
            a_table.get("default"),
        );
        let (b_type, b_prompt) = (b_table.get("type"), b_table.get("prompt"));

        assert_eq!(a_type, Some(&Value::String("bool".to_string())));
        assert_eq!(a_prompt, Some(&Value::String("foo".to_string())));
        assert_eq!(a_default, Some(&Value::Boolean(false)));

        assert_eq!(b_type, Some(&Value::String("string".to_string())));
        assert_eq!(b_prompt, Some(&Value::String("bar".to_string())));
    }
}
