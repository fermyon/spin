//! Spin manifest errors

/// Spin manifest errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid digest format.
    #[error("invalid digest {0:?}: {1}")]
    InvalidDigest(String, String),

    /// Invalid ID format.
    #[error("invalid ID `{id}`: {reason}")]
    InvalidID {
        /// The invalid ID
        id: String,
        /// The reason why the ID is invalid
        reason: String,
    },

    /// Invalid trigger config
    #[error("invalid `{trigger_type}` trigger config: {reason}")]
    InvalidTriggerConfig {
        /// The trigger type
        trigger_type: String,
        /// The reason why the config is invalid
        reason: String,
    },

    /// Invalid variable definition
    #[error("invalid variable definition for `{name}`: {reason}")]
    InvalidVariable {
        /// The invalid variable name
        name: String,
        /// The reason why the variable is invalid
        reason: String,
    },

    /// Invalid manifest version
    #[error("invalid manifest version: {0}")]
    InvalidVersion(String),

    /// IO error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Error serializing metadata
    #[error("error serializing metadata: {0}")]
    MetadataSerialization(String),

    /// Error parsing TOML
    #[error(transparent)]
    TomlParse(#[from] toml::de::Error),

    /// Validation error
    #[error(transparent)]
    ValidationError(anyhow::Error),
}
