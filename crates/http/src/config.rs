use serde::{Deserialize, Serialize};

/// Configuration for the HTTP trigger
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpTriggerConfig {
    /// Component ID to invoke
    pub component: String,
    /// HTTP route the component will be invoked for
    pub route: String,
    /// The HTTP executor the component requires
    #[serde(default)]
    pub executor: Option<HttpExecutorType>,
}

/// The executor for the HTTP component.
/// The component can either implement the Spin HTTP interface,
/// or the Wagi CGI interface.
///
/// If an executor is not specified, the inferred default is `HttpExecutor::Spin`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase", tag = "type")]
pub enum HttpExecutorType {
    /// The component implements the Spin HTTP interface.
    #[default]
    Spin,
    /// The component implements the Wagi CGI interface.
    Wagi(WagiTriggerConfig),
}

/// Wagi specific configuration for the http executor.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WagiTriggerConfig {
    /// The name of the entrypoint.
    #[serde(default)]
    pub entrypoint: String,

    /// A string representation of the argv array.
    ///
    /// This should be a space-separate list of strings. The value
    /// ${SCRIPT_NAME} will be replaced with the Wagi SCRIPT_NAME,
    /// and the value ${ARGS} will be replaced with the query parameter
    /// name/value pairs presented as args. For example,
    /// `param1=val1&param2=val2` will become `param1=val1 param2=val2`,
    /// which will then be presented to the program as two arguments
    /// in argv.
    #[serde(default)]
    pub argv: String,
}

impl Default for WagiTriggerConfig {
    fn default() -> Self {
        /// This is the default Wagi entrypoint.
        const WAGI_DEFAULT_ENTRYPOINT: &str = "_start";
        const WAGI_DEFAULT_ARGV: &str = "${SCRIPT_NAME} ${ARGS}";

        Self {
            entrypoint: WAGI_DEFAULT_ENTRYPOINT.to_owned(),
            argv: WAGI_DEFAULT_ARGV.to_owned(),
        }
    }
}
