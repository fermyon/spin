/// A specialized Result type for publish operations
pub type PublishResult<T> = std::result::Result<T, PublishError>;

/// Describes various errors that can be returned during publishing
#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    /// The bindle already exists
    #[error("Bindle {0} already exists on the server")]
    BindleAlreadyExists(String),
    /// Bindle client failure
    #[error("Error creating bindle client")]
    BindleClient(#[from] bindle::client::ClientError),
    /// Malformed bindle id
    #[error("App name and version '{0}' do not form a bindle ID")]
    BindleId(String),
    /// Malformed bindle id
    #[error("App name '{0}' contains characters not allowed in a bindle name. A bindle name may contain only letters, numbers, and underscores")]
    BindleNameInvalidChars(String),
    /// Publishing of components whose sources are already bindles is not supported
    #[error("This version of Spin can't publish components whose sources are already bindles")]
    BindlePushingNotImplemented,
    /// IO errors from interacting with the file system
    #[error("{description}")]
    Io {
        /// Error description
        description: String,
        /// Underlying lower level error that caused your error
        source: std::io::Error,
    },
    /// Build artifact is missing
    #[error("Missing build artifact: '{0}'")]
    MissingBuildArtifact(String),
    /// Invalid TOML serialization that can occur when serializing an object to a request
    #[error("{description}")]
    TomlSerialization {
        /// Error description
        description: String,
        /// Underlying lower level error that caused your error
        source: toml::ser::Error,
    },
    /// A catch-all for anyhow errors.
    /// Contains an error message describing the underlying issue.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
