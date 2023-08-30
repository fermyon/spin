use anyhow::ensure;
use serde::{Deserialize, Serialize};
use spin_manifest::Variable;

/// Variable configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawVariable {
    /// If set, this variable is required; may not be set with `default`.
    #[serde(default)]
    pub required: bool,
    /// If set, the default value for this variable; may not be set with `required`.
    #[serde(default)]
    pub default: Option<String>,
    /// If set, this variable should be treated as sensitive.
    #[serde(default)]
    pub secret: bool,
}

impl TryFrom<RawVariable> for Variable {
    type Error = anyhow::Error;

    fn try_from(var: RawVariable) -> Result<Self, Self::Error> {
        ensure!(
            var.required ^ var.default.is_some(),
            "variable should either have `required` set to true OR have a non-empty default value"
        );
        Ok(Variable {
            default: var.default,
            secret: var.secret,
        })
    }
}
