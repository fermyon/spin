use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::{Diagnosis, Diagnostic, PatientApp};

/// ComponentConfigDiagnostic detects problems with component `config` entries.
#[derive(Default)]
pub struct ComponentConfigDiagnostic;

#[async_trait]
impl Diagnostic for ComponentConfigDiagnostic {
    type Diagnosis = ComponentConfigDiagnosis;

    async fn diagnose(&self, patient: &PatientApp) -> Result<Vec<Self::Diagnosis>> {
        let variables = match patient
            .manifest_doc
            .get("variables")
            .and_then(|item| item.as_table())
        {
            None => vec![],
            Some(table) => app_variables(table),
        };

        let component_configs = match patient
            .manifest_doc
            .get("component")
            .and_then(|item| item.as_array_of_tables())
        {
            None => vec![],
            Some(arr) => arr.iter().filter_map(component_config).collect(),
        };

        if spin_config::Resolver::new(variables.clone()).is_err() {
            // TODO: It eould be nice to be able to continue even if a variable failed
            // to validate, but that requires more of the internals of spin_config than
            // it currently exposes.
            return Ok(vec![]);
        }

        let diagnoses = component_configs
            .into_iter()
            .flat_map(|(id, cfg)| diagnose_component(variables.clone(), id, cfg))
            .collect();

        Ok(diagnoses)
    }
}

fn app_variables(table: &toml_edit::Table) -> Vec<(String, spin_app::Variable)> {
    table
        .iter()
        .map(|(k, _)| {
            (
                k.to_owned(),
                spin_app::Variable {
                    default: None,
                    secret: false,
                },
            )
        })
        .collect()
}

fn component_config(table: &toml_edit::Table) -> Option<(String, HashMap<String, String>)> {
    let Some(id) = table.get("id").and_then(|item| item.as_str()) else {
        return None;
    };

    let Some(cfg_table) = table.get("config").and_then(|item| item.as_table()) else {
        return None;
    };

    let configs = cfg_table
        .iter()
        .flat_map(|(k, v)| {
            v.as_str()
                .map(|template| (k.to_owned(), template.to_owned()))
        })
        .collect();

    Some((id.to_owned(), configs))
}

fn diagnose_component(
    variables: Vec<(String, spin_app::Variable)>,
    component_id: String,
    cfg: HashMap<String, String>,
) -> impl Iterator<Item = ComponentConfigDiagnosis> {
    cfg.into_iter().filter_map(move |(key, t)| {
        let mut resolver = spin_config::Resolver::new(variables.clone()).unwrap(); // Safe to unwrap because the caller checks the variables will go into a Resolver
        resolver
            .add_component_config(&component_id, vec![(key.clone(), t)])
            .err()
            .and_then(|e| ComponentConfigDiagnosis::try_create(&component_id, &key, e))
    })
}

/// A problem with a component configuration entry.
#[derive(Debug)]
pub struct ComponentConfigDiagnosis {
    inner: ComponentConfigDiagnosisInner,
}

impl ComponentConfigDiagnosis {
    fn try_create(component_id: &str, key: &str, e: spin_config::Error) -> Option<Self> {
        ComponentConfigDiagnosisInner::try_create(component_id, key, e).map(|inner| Self { inner })
    }
}

// TODO: It would be nice to pick out invalid variable references, so we could
// report all of them, but that would require exposing the expression parser.
// The resolver turns both syntax errors and bad references into "InvalidTemplate".
// But the message distinguishes the cause clearly.
#[derive(Debug)]
enum ComponentConfigDiagnosisInner {
    InvalidKey {
        component_id: String,
        key: String,
        reason: String,
    },
    InvalidTemplate {
        component_id: String,
        key: String,
        reason: String,
    },
}

impl ComponentConfigDiagnosisInner {
    fn try_create(component_id: &str, key: &str, e: spin_config::Error) -> Option<Self> {
        let component_id = component_id.to_owned();
        let key = key.to_owned();
        match e {
            spin_config::Error::InvalidKey(_, reason) => Some(Self::InvalidKey {
                component_id,
                key,
                reason,
            }),
            spin_config::Error::InvalidTemplate(reason) => Some(Self::InvalidTemplate {
                component_id,
                key,
                reason,
            }),
            _ => None,
        }
    }
}

impl Diagnosis for ComponentConfigDiagnosis {
    // TODO: These are not readily treatable. We could suggest 'nearby' variables for bad
    // references, but again that requires a lot of access to template internals, and would
    // be a bit ad hoc anyway!
    fn description(&self) -> String {
        match &self.inner {
            ComponentConfigDiagnosisInner::InvalidKey {
                component_id,
                key,
                reason,
            } => format!("config key '{key}' in component '{component_id}' is invalid: {reason}"),
            ComponentConfigDiagnosisInner::InvalidTemplate {
                component_id,
                key,
                reason,
            } => format!(
                "config entry '{key}' in component '{component_id}' has invalid template: {reason}"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::{run_correct_test, run_untreatable_test};

    use super::*;

    #[tokio::test]
    async fn test_correct() {
        run_correct_test::<ComponentConfigDiagnostic>("component_config").await;
    }

    #[tokio::test]
    async fn test_bad_key() {
        let diag =
            run_untreatable_test::<ComponentConfigDiagnostic>("component_config", "bad_key").await;
        assert!(matches!(
            diag.inner,
            ComponentConfigDiagnosisInner::InvalidKey { .. }
        ));
    }

    #[tokio::test]
    async fn test_bad_variable_ref() {
        let diag =
            run_untreatable_test::<ComponentConfigDiagnostic>("component_config", "bad_ref").await;
        assert!(matches!(
            diag.inner,
            ComponentConfigDiagnosisInner::InvalidTemplate { .. }
        ));
    }

    #[tokio::test]
    async fn test_bad_template_syntax() {
        let diag =
            run_untreatable_test::<ComponentConfigDiagnostic>("component_config", "bad_syntax")
                .await;
        assert!(matches!(
            diag.inner,
            ComponentConfigDiagnosisInner::InvalidTemplate { .. }
        ));
    }
}
