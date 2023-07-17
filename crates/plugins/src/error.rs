pub type PluginLookupResult<T> = std::result::Result<T, Error>;

/// Error message during plugin lookup or deserializing
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    NotFound(NotFoundError),

    #[error("{0}")]
    ConnectionFailed(ConnectionFailedError),

    #[error("{0}")]
    InvalidManifest(InvalidManifestError),

    #[error("URL parse error {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Contains error details for when a plugin resource cannot be found at expected location
#[derive(Debug)]
pub struct NotFoundError {
    name: Option<String>,
    addr: String,
    err: String,
}

impl NotFoundError {
    pub fn new(name: Option<String>, addr: String, err: String) -> Self {
        Self { name, addr, err }
    }
}

impl std::fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "plugin '{}' not found at expected location {}: {}",
            self.name.as_deref().unwrap_or_default(),
            self.addr,
            self.err
        ))
    }
}

/// Contains error details for when a plugin manifest cannot be properly serialized
#[derive(Debug)]
pub struct InvalidManifestError {
    name: Option<String>,
    addr: String,
    err: String,
}

impl InvalidManifestError {
    pub fn new(name: Option<String>, addr: String, err: String) -> Self {
        Self { name, addr, err }
    }
}

impl std::fmt::Display for InvalidManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "invalid manifest for plugin '{}' at {}: {}",
            self.name.clone().unwrap_or_default(),
            self.addr,
            self.err
        ))
    }
}

/// Contains error details for when there is an error getting a plugin resource from an address.
#[derive(Debug)]
pub struct ConnectionFailedError {
    addr: String,
    err: String,
}

impl ConnectionFailedError {
    pub fn new(addr: String, err: String) -> Self {
        Self { addr, err }
    }
}

impl std::fmt::Display for ConnectionFailedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "failed to connect to endpoint {}: {}",
            self.addr, self.err
        ))
    }
}
