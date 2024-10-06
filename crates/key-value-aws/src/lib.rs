mod store;

use serde::Deserialize;
use spin_factor_key_value::runtime_config::spin::MakeKeyValueStore;
use store::{
    KeyValueAwsDynamo, KeyValueAwsDynamoAuthOptions, KeyValueAwsDynamoRuntimeConfigOptions,
};

/// A key-value store that uses AWS Dynamo as the backend.
#[derive(Default)]
pub struct AwsKeyValueStore {
    _priv: (),
}

impl AwsKeyValueStore {
    /// Creates a new `AwsKeyValueStore`.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Runtime configuration for the AWS Dynamo key-value store.
#[derive(Deserialize)]
pub struct AwsDynamoKeyValueRuntimeConfig {
    /// The access key for the AWS Dynamo DB account role.
    access_key: Option<String>,
    /// The secret key for authorization on the AWS Dynamo DB account.
    secret_key: Option<String>,
    /// The token for authorization on the AWS Dynamo DB account.
    token: Option<String>,
    /// The AWS region where the database is located
    region: String,
    /// The AWS Dynamo DB table.
    table: String,
}

impl MakeKeyValueStore for AwsKeyValueStore {
    const RUNTIME_CONFIG_TYPE: &'static str = "aws_dynamo";

    type RuntimeConfig = AwsDynamoKeyValueRuntimeConfig;

    type StoreManager = KeyValueAwsDynamo;

    fn make_store(
        &self,
        runtime_config: Self::RuntimeConfig,
    ) -> anyhow::Result<Self::StoreManager> {
        let AwsDynamoKeyValueRuntimeConfig {
            access_key,
            secret_key,
            token,
            region,
            table,
        } = runtime_config;
        let auth_options = match (access_key, secret_key) {
            (Some(access_key), Some(secret_key)) => {
                KeyValueAwsDynamoAuthOptions::RuntimeConfigValues(
                    KeyValueAwsDynamoRuntimeConfigOptions::new(access_key, secret_key, token),
                )
            }
            _ => KeyValueAwsDynamoAuthOptions::Environmental,
        };
        KeyValueAwsDynamo::new(region, table, auth_options)
    }
}
