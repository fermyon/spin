use crate::{config::Parameters, emoji};
use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use liquid_core::Value;
use regex::Regex;
use std::ops::Index;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ConversionError {
    #[error("parameter `{parameter}` of placeholder `{var_name}` should be a `{correct_type}`")]
    WrongTypeParameter {
        var_name: String,
        parameter: String,
        correct_type: String,
    },
    #[error("placeholder `{var_name}` should be a table")]
    InvalidPlaceholderFormat { var_name: String },
    #[error("missing prompt question for `{var_name}`")]
    MissingPrompt { var_name: String },
    #[error("choices array empty for `{var_name}`")]
    EmptyChoices { var_name: String },
    #[error("default is `{default}`, but is not a valid value in choices array `{choices:?}` for  `{var_name}`")]
    InvalidDefault {
        var_name: String,
        default: String,
        choices: Vec<String>,
    },
    #[error(
        "invalid type for variable `{var_name}`: `{value}` possible values are `bool` and `string`"
    )]
    InvalidVariableType { var_name: String, value: String },
    #[error("bool type does not support `choices` field")]
    ChoicesOnBool { var_name: String },
    #[error("bool type does not support `regex` field")]
    RegexOnBool { var_name: String },
    #[error("variable `{var_name}` was missing in config file running on silent mode")]
    MissingPlaceholderVariable { var_name: String },
    #[error("field `{field}` of variable `{var_name}` does not match configured regex")]
    RegexDoesntMatchField { var_name: String, field: String },
    #[error("regex of `{var_name}` is not a valid regex. {error}")]
    InvalidRegex {
        var_name: String,
        regex: String,
        error: regex::Error,
    },
}

pub(crate) struct Variables(pub Vec<Variable>);

impl TryFrom<&Parameters> for Variables {
    type Error = anyhow::Error;

    fn try_from(parameters: &Parameters) -> Result<Self, Self::Error> {
        try_from_parameters(parameters).map(Variables)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum VariableInfo {
    Bool { default: Option<bool> },
    String { entry: Box<StringEntry> },
}

#[derive(Debug, Clone)]
pub(crate) struct StringEntry {
    pub default: Option<String>,
    pub choices: Option<Vec<String>>,
    pub regex: Option<Regex>,
}

#[derive(Debug)]
pub(crate) struct Variable {
    pub var_info: VariableInfo,
    pub var_name: String,
    pub prompt: String,
}

impl Variable {
    pub(crate) fn prompt(&self) -> Result<String> {
        let prompt = format!("{} {}", emoji::SHRUG, style(&self.prompt).bold(),);
        match &self.var_info {
            VariableInfo::Bool { default } => {
                let choices = [false.to_string(), true.to_string()];
                let chosen = Select::with_theme(&ColorfulTheme::default())
                    .items(&choices)
                    .with_prompt(&prompt)
                    .default(if default.unwrap_or(false) { 1 } else { 0 })
                    .interact()?;

                Ok(choices.index(chosen).to_string())
            }
            VariableInfo::String { entry } => match &entry.choices {
                Some(choices) => {
                    let default = entry
                        .default
                        .as_ref()
                        .map_or(0, |default| choices.binary_search(default).unwrap_or(0));
                    let chosen = Select::with_theme(&ColorfulTheme::default())
                        .items(choices)
                        .with_prompt(&prompt)
                        .default(default)
                        .interact()?;

                    Ok(choices.index(chosen).to_string())
                }
                None => {
                    let prompt = format!(
                        "{} {}",
                        prompt,
                        match &entry.default {
                            Some(d) => format!("[default: {}]", style(d).bold()),
                            None => "".into(),
                        }
                    );
                    let default = entry.default.as_ref().map(|v| v.into());

                    match &entry.regex {
                        Some(regex) => loop {
                            let user_entry = user_question(prompt.as_str(), &default)?;
                            if regex.is_match(&user_entry) {
                                break Ok(user_entry);
                            }
                            eprintln!(
                                "{} {} \"{}\" {}",
                                emoji::WARN,
                                style("Sorry,").bold().red(),
                                style(&user_entry).bold().yellow(),
                                style(format!("is not a valid value for {}", self.var_name))
                                    .bold()
                                    .red()
                            );
                        },
                        None => Ok(user_question(prompt.as_str(), &default)?),
                    }
                }
            },
        }
    }

    pub(crate) fn resolve(&self, val: Option<&str>) -> Result<Value> {
        let val = val
            .map(|v| Ok(v.to_string()))
            .unwrap_or_else(|| self.prompt())?;
        self.as_value(val)
    }

    fn as_value(&self, user_entry: String) -> Result<Value> {
        match self.var_info {
            VariableInfo::Bool { .. } => {
                let as_bool = user_entry.parse::<bool>()?; // this shouldn't fail if checked before
                Ok(Value::Scalar(as_bool.into()))
            }
            VariableInfo::String { .. } => Ok(Value::Scalar(user_entry.into())),
        }
    }
}

fn user_question(prompt: &str, default: &Option<String>) -> Result<String> {
    let mut i = Input::<String>::new();
    i.with_prompt(prompt.to_string());
    if let Some(s) = default {
        i.default(s.to_owned());
    }
    i.interact().map_err(Into::<anyhow::Error>::into)
}

pub(crate) fn try_from_parameters(parameters: &Parameters) -> Result<Vec<Variable>> {
    let mut slots = Vec::with_capacity(parameters.len());
    for (k, v) in parameters.iter() {
        slots.push(try_from_kv(k, v)?);
    }
    Ok(slots)
}

fn try_from_kv(k: &str, v: &toml::Value) -> Result<Variable, ConversionError> {
    let table = v
        .as_table()
        .ok_or(ConversionError::InvalidPlaceholderFormat {
            var_name: k.to_string(),
        })?;

    let var_type = extract_type(k, table.get("type"))?;
    let regex = extract_regex(k, var_type, table.get("regex"))?;
    let prompt = extract_prompt(k, table.get("prompt"))?;
    let choices = extract_choices(k, var_type, regex.as_ref(), table.get("choices"))?;
    let default_choice = extract_default(
        k,
        var_type,
        regex.as_ref(),
        table.get("default"),
        choices.as_ref(),
    )?;

    let var_info = match (var_type, default_choice) {
        (SupportedVarType::Bool, Some(SupportedVarValue::Bool(value))) => VariableInfo::Bool {
            default: Some(value),
        },
        (SupportedVarType::String, Some(SupportedVarValue::String(value))) => {
            VariableInfo::String {
                entry: Box::new(StringEntry {
                    default: Some(value),
                    choices,
                    regex,
                }),
            }
        }
        (SupportedVarType::Bool, None) => VariableInfo::Bool { default: None },
        (SupportedVarType::String, None) => VariableInfo::String {
            entry: Box::new(StringEntry {
                default: None,
                choices,
                regex,
            }),
        },
        _ => unreachable!("It should not have come to this..."),
    };
    Ok(Variable {
        var_name: k.to_string(),
        var_info,
        prompt,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum SupportedVarValue {
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SupportedVarType {
    Bool,
    String,
}

fn extract_regex(
    var_name: &str,
    var_type: SupportedVarType,
    table_entry: Option<&toml::Value>,
) -> Result<Option<Regex>, ConversionError> {
    match (var_type, table_entry) {
        (SupportedVarType::Bool, Some(_)) => Err(ConversionError::RegexOnBool {
            var_name: var_name.into(),
        }),
        (SupportedVarType::String, Some(toml::Value::String(value))) => match Regex::new(value) {
            Ok(regex) => Ok(Some(regex)),
            Err(e) => Err(ConversionError::InvalidRegex {
                var_name: var_name.into(),
                regex: value.clone(),
                error: e,
            }),
        },
        (SupportedVarType::String, Some(_)) => Err(ConversionError::WrongTypeParameter {
            var_name: var_name.into(),
            parameter: "regex".to_string(),
            correct_type: "String".to_string(),
        }),
        (_, None) => Ok(None),
    }
}

fn extract_type(
    var_name: &str,
    table_entry: Option<&toml::Value>,
) -> Result<SupportedVarType, ConversionError> {
    match table_entry {
        None => Ok(SupportedVarType::String),
        Some(toml::Value::String(value)) if value == "string" => Ok(SupportedVarType::String),
        Some(toml::Value::String(value)) if value == "bool" => Ok(SupportedVarType::Bool),
        Some(toml::Value::String(value)) => Err(ConversionError::InvalidVariableType {
            var_name: var_name.into(),
            value: value.clone(),
        }),
        Some(_) => Err(ConversionError::WrongTypeParameter {
            var_name: var_name.into(),
            parameter: "type".to_string(),
            correct_type: "String".to_string(),
        }),
    }
}

fn extract_prompt(
    var_name: &str,
    table_entry: Option<&toml::Value>,
) -> Result<String, ConversionError> {
    match table_entry {
        Some(toml::Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(ConversionError::WrongTypeParameter {
            var_name: var_name.into(),
            parameter: "prompt".into(),
            correct_type: "String".into(),
        }),
        None => Err(ConversionError::MissingPrompt {
            var_name: var_name.into(),
        }),
    }
}

fn extract_default(
    var_name: &str,
    var_type: SupportedVarType,
    regex: Option<&Regex>,
    table_entry: Option<&toml::Value>,
    choices: Option<&Vec<String>>,
) -> Result<Option<SupportedVarValue>, ConversionError> {
    match (table_entry, choices, var_type) {
        // no default set
        (None, _, _) => Ok(None),
        // default set without choices
        (Some(toml::Value::Boolean(value)), _, SupportedVarType::Bool) => {
            Ok(Some(SupportedVarValue::Bool(*value)))
        }
        (Some(toml::Value::String(value)), None, SupportedVarType::String) => {
            if let Some(reg) = regex {
                if !reg.is_match(value) {
                    return Err(ConversionError::RegexDoesntMatchField {
                        var_name: var_name.into(),
                        field: "default".to_string(),
                    });
                }
            }
            Ok(Some(SupportedVarValue::String(value.clone())))
        }

        // default and choices set
        // No need to check bool because it always has a choices vec with two values
        (Some(toml::Value::String(value)), Some(choices), SupportedVarType::String) => {
            if !choices.contains(value) {
                Err(ConversionError::InvalidDefault {
                    var_name: var_name.into(),
                    default: value.clone(),
                    choices: choices.clone(),
                })
            } else {
                if let Some(reg) = regex {
                    if !reg.is_match(value) {
                        return Err(ConversionError::RegexDoesntMatchField {
                            var_name: var_name.into(),
                            field: "default".to_string(),
                        });
                    }
                }
                Ok(Some(SupportedVarValue::String(value.clone())))
            }
        }

        // Wrong type of variables
        (Some(_), _, type_name) => Err(ConversionError::WrongTypeParameter {
            var_name: var_name.into(),
            parameter: "default".to_string(),
            correct_type: match type_name {
                SupportedVarType::Bool => "bool".to_string(),
                SupportedVarType::String => "string".to_string(),
            },
        }),
    }
}

fn extract_choices(
    var_name: &str,
    var_type: SupportedVarType,
    regex: Option<&Regex>,
    table_entry: Option<&toml::Value>,
) -> Result<Option<Vec<String>>, ConversionError> {
    match (table_entry, var_type) {
        (None, SupportedVarType::Bool) => Ok(None),
        (Some(_), SupportedVarType::Bool) => Err(ConversionError::ChoicesOnBool {
            var_name: var_name.into(),
        }),
        (Some(toml::Value::Array(arr)), SupportedVarType::String) if arr.is_empty() => {
            Err(ConversionError::EmptyChoices {
                var_name: var_name.into(),
            })
        }
        (Some(toml::Value::Array(arr)), SupportedVarType::String) => {
            // Checks if very entry in the array is a String
            let converted = arr
                .iter()
                .map(|entry| match entry {
                    toml::Value::String(s) => Ok(s.clone()),
                    _ => Err(()),
                })
                .collect::<Vec<_>>();
            if converted.iter().any(|v| v.is_err()) {
                return Err(ConversionError::WrongTypeParameter {
                    var_name: var_name.into(),
                    parameter: "choices".to_string(),
                    correct_type: "String Array".to_string(),
                });
            }

            let strings = converted
                .iter()
                .cloned()
                .map(|v| v.unwrap())
                .collect::<Vec<_>>();
            // check if regex matches every choice
            if let Some(reg) = regex {
                if strings.iter().any(|v| !reg.is_match(v)) {
                    return Err(ConversionError::RegexDoesntMatchField {
                        var_name: var_name.into(),
                        field: "choices".to_string(),
                    });
                }
            }

            Ok(Some(strings))
        }
        (Some(_), SupportedVarType::String) => Err(ConversionError::WrongTypeParameter {
            var_name: var_name.into(),
            parameter: "choices".to_string(),
            correct_type: "String Array".to_string(),
        }),
        (None, SupportedVarType::String) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_choices_boolean() {
        let result = extract_choices("foo", SupportedVarType::Bool, None, None);

        assert_eq!(result, Ok(None));
    }

    #[test]
    fn boolean_cant_have_choices() {
        let result = extract_choices(
            "foo",
            SupportedVarType::Bool,
            None,
            Some(&toml::Value::Array(vec![
                toml::Value::Boolean(true),
                toml::Value::Boolean(false),
            ])),
        );

        assert_eq!(
            result,
            Err(ConversionError::ChoicesOnBool {
                var_name: "foo".into()
            })
        );
    }

    #[test]
    fn choices_cant_be_an_empty_array() {
        let result = extract_choices(
            "foo",
            SupportedVarType::String,
            None,
            Some(&toml::Value::Array(Vec::new())),
        );

        assert_eq!(
            result,
            Err(ConversionError::EmptyChoices {
                var_name: "foo".into()
            })
        );
    }

    #[test]
    fn choices_array_cant_have_anything_but_strings() {
        let result = extract_choices(
            "foo",
            SupportedVarType::String,
            None,
            Some(&toml::Value::Array(vec![
                toml::Value::String("bar".into()),
                toml::Value::Boolean(false),
            ])),
        );

        assert_eq!(
            result,
            Err(ConversionError::WrongTypeParameter {
                var_name: "foo".into(),
                parameter: "choices".into(),
                correct_type: "String Array".into()
            })
        );
    }

    #[test]
    fn choices_is_array_string_no_regex_is_fine() {
        let result = extract_choices(
            "foo",
            SupportedVarType::String,
            None,
            Some(&toml::Value::Array(vec![
                toml::Value::String("bar".into()),
                toml::Value::String("zoo".into()),
            ])),
        );

        assert_eq!(result, Ok(Some(vec!["bar".to_string(), "zoo".to_string()])));
    }

    #[test]
    fn choices_is_array_string_that_doesnt_match_regex_is_error() {
        let valid_ident = regex::Regex::new(r"^([a-zA-Z][a-zA-Z0-9_-]+)$").unwrap();

        let result = extract_choices(
            "foo",
            SupportedVarType::String,
            Some(&valid_ident),
            Some(&toml::Value::Array(vec![
                toml::Value::String("0bar".into()),
                toml::Value::String("zoo".into()),
            ])),
        );

        assert_eq!(
            result,
            Err(ConversionError::RegexDoesntMatchField {
                var_name: "foo".into(),
                field: "choices".into()
            })
        );
    }

    #[test]
    fn choices_is_array_string_that_all_match_regex_is_good() {
        let valid_ident = regex::Regex::new(r"^([a-zA-Z][a-zA-Z0-9_-]+)$").unwrap();

        let result = extract_choices(
            "foo",
            SupportedVarType::String,
            Some(&valid_ident),
            Some(&toml::Value::Array(vec![
                toml::Value::String("bar0".into()),
                toml::Value::String("zoo".into()),
            ])),
        );

        assert_eq!(
            result,
            Ok(Some(vec!["bar0".to_string(), "zoo".to_string()]))
        );
    }

    #[test]
    fn choices_is_not_array_string_is_error() {
        let result = extract_choices(
            "foo",
            SupportedVarType::String,
            None,
            Some(&toml::Value::String("bar".into())),
        );

        assert_eq!(
            result,
            Err(ConversionError::WrongTypeParameter {
                var_name: "foo".into(),
                parameter: "choices".into(),
                correct_type: "String Array".into()
            })
        );
    }

    #[test]
    fn no_choices_for_type_string() {
        let result = extract_choices("foo", SupportedVarType::String, None, None);

        assert_eq!(result, Ok(None));
    }

    #[test]
    fn empty_default_is_fine() {
        let result = extract_default("foo", SupportedVarType::String, None, None, None);

        assert_eq!(result, Ok(None));
    }

    #[test]
    fn default_for_boolean_is_fine() {
        let result = extract_default(
            "foo",
            SupportedVarType::Bool,
            None,
            Some(&toml::Value::Boolean(true)),
            None,
        );

        assert_eq!(result, Ok(Some(SupportedVarValue::Bool(true))))
    }

    #[test]
    fn default_for_string_with_no_choices_and_no_regex() {
        let result = extract_default(
            "foo",
            SupportedVarType::String,
            None,
            Some(&toml::Value::String("bar".to_string())),
            None,
        );

        assert_eq!(
            result,
            Ok(Some(SupportedVarValue::String("bar".to_string())))
        )
    }

    #[test]
    fn default_for_string_with_no_choices_and_matching_regex() {
        let valid_ident = regex::Regex::new(r"^([a-zA-Z][a-zA-Z0-9_-]+)$").unwrap();

        let result = extract_default(
            "foo",
            SupportedVarType::String,
            Some(&valid_ident),
            Some(&toml::Value::String("bar".to_string())),
            None,
        );

        assert_eq!(
            result,
            Ok(Some(SupportedVarValue::String("bar".to_string())))
        )
    }

    #[test]
    fn default_for_string_with_no_choices_and_regex_doesnt_match() {
        let valid_ident = regex::Regex::new(r"^([a-zA-Z][a-zA-Z0-9_-]+)$").unwrap();

        let result = extract_default(
            "foo",
            SupportedVarType::String,
            Some(&valid_ident),
            Some(&toml::Value::String("0bar".to_string())),
            None,
        );

        assert_eq!(
            result,
            Err(ConversionError::RegexDoesntMatchField {
                var_name: "foo".into(),
                field: "default".into()
            })
        )
    }

    #[test]
    fn default_for_string_isnt_on_choices() {
        let result = extract_default(
            "foo",
            SupportedVarType::String,
            None,
            Some(&toml::Value::String("bar".to_string())),
            Some(&vec!["zoo".to_string(), "far".to_string()]),
        );

        assert_eq!(
            result,
            Err(ConversionError::InvalidDefault {
                var_name: "foo".into(),
                default: "bar".into(),
                choices: vec!["zoo".to_string(), "far".to_string()]
            })
        )
    }

    #[test]
    fn default_for_string_is_on_choices() {
        let result = extract_default(
            "foo",
            SupportedVarType::String,
            None,
            Some(&toml::Value::String("bar".to_string())),
            Some(&vec!["zoo".to_string(), "bar".to_string()]),
        );

        assert_eq!(result, Ok(Some(SupportedVarValue::String("bar".into()))))
    }

    #[test]
    fn default_for_string_is_on_choices_and_matches_regex() {
        let valid_ident = regex::Regex::new(r"^([a-zA-Z][a-zA-Z0-9_-]+)$").unwrap();

        let result = extract_default(
            "foo",
            SupportedVarType::String,
            Some(&valid_ident),
            Some(&toml::Value::String("bar".to_string())),
            Some(&vec!["zoo".to_string(), "bar".to_string()]),
        );

        assert_eq!(result, Ok(Some(SupportedVarValue::String("bar".into()))))
    }

    #[test]
    fn default_for_string_only_accepts_strings() {
        let result = extract_default(
            "foo",
            SupportedVarType::String,
            None,
            Some(&toml::Value::Integer(0)),
            None,
        );

        assert_eq!(
            result,
            Err(ConversionError::WrongTypeParameter {
                var_name: "foo".into(),
                parameter: "default".into(),
                correct_type: "string".into()
            })
        )
    }

    #[test]
    fn default_for_bool_only_accepts_bool() {
        let result = extract_default(
            "foo",
            SupportedVarType::Bool,
            None,
            Some(&toml::Value::Integer(0)),
            None,
        );

        assert_eq!(
            result,
            Err(ConversionError::WrongTypeParameter {
                var_name: "foo".into(),
                parameter: "default".into(),
                correct_type: "bool".into()
            })
        )
    }

    #[test]
    fn prompt_cant_be_empty() {
        let result = extract_prompt("foo", None);

        assert_eq!(
            result,
            Err(ConversionError::MissingPrompt {
                var_name: "foo".into(),
            })
        )
    }

    #[test]
    fn prompt_must_be_string() {
        let result = extract_prompt("foo", Some(&toml::Value::Integer(0)));

        assert_eq!(
            result,
            Err(ConversionError::WrongTypeParameter {
                var_name: "foo".into(),
                parameter: "prompt".into(),
                correct_type: "String".into()
            })
        )
    }

    #[test]
    fn prompt_as_string_is_ok() {
        let result = extract_prompt("foo", Some(&toml::Value::String("hello world".into())));

        assert_eq!(result, Ok("hello world".into()))
    }

    #[test]
    fn empty_type_is_string() {
        let result = extract_type("foo", None);

        assert_eq!(result, Ok(SupportedVarType::String));
    }

    #[test]
    fn type_must_be_string_type() {
        let result = extract_type("foo", Some(&toml::Value::Integer(0)));

        assert_eq!(
            result,
            Err(ConversionError::WrongTypeParameter {
                var_name: "foo".into(),
                parameter: "type".into(),
                correct_type: "String".into()
            })
        );
    }

    #[test]
    fn type_must_either_be_string_or_bool() {
        let result_bool = extract_type("foo", Some(&toml::Value::String("bool".into())));
        let result_string = extract_type("foo", Some(&toml::Value::String("string".into())));
        let result_err = extract_type("foo", Some(&toml::Value::String("bar".into())));

        assert_eq!(result_bool, Ok(SupportedVarType::Bool));
        assert_eq!(result_string, Ok(SupportedVarType::String));
        assert_eq!(
            result_err,
            Err(ConversionError::InvalidVariableType {
                var_name: "foo".into(),
                value: "bar".into()
            })
        )
    }

    #[test]
    fn bools_cant_have_regex() {
        let result = extract_regex(
            "foo",
            SupportedVarType::Bool,
            Some(&toml::Value::String("".into())),
        );

        assert!(result.is_err())
    }

    #[test]
    fn no_regex_is_ok() {
        let result_bool = extract_regex("foo", SupportedVarType::Bool, None);
        let result_string = extract_regex("foo", SupportedVarType::String, None);

        assert!(result_bool.is_ok());
        assert!(result_string.is_ok())
    }

    #[test]
    fn strings_can_have_regex() {
        let result = extract_regex(
            "foo",
            SupportedVarType::String,
            Some(&toml::Value::String("^([a-zA-Z][a-zA-Z0-9_-]+)$".into())),
        );

        assert!(result.is_ok())
    }

    #[test]
    fn invalid_regex_is_err() {
        let result = extract_regex(
            "foo",
            SupportedVarType::String,
            Some(&toml::Value::String("*".into())),
        );

        assert!(result.is_err())
    }

    #[test]
    fn only_tables_as_placeholder_values() {
        let result = try_from_kv("foo", &toml::Value::Integer(Default::default()));

        assert!(result.is_err());
        let result = result.err().unwrap();
        assert_eq!(
            result,
            ConversionError::InvalidPlaceholderFormat {
                var_name: "foo".into()
            }
        );
    }
}
