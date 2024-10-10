use std::sync::Arc;

use anyhow::Result;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_dynamodb::{
    config::{ProvideCredentials, SharedCredentialsProvider},
    primitives::Blob,
    types::AttributeValue,
    Client,
};
use spin_core::async_trait;
use spin_factor_key_value::{log_error, Error, Store, StoreManager};
use tracing::{instrument, Level};

pub struct KeyValueAwsDynamo {
    table: String,
    client: Client,
}

/// AWS Dynamo Key / Value runtime config literal options for authentication
#[derive(Clone, Debug)]
pub struct KeyValueAwsDynamoRuntimeConfigOptions {
    access_key: String,
    secret_key: String,
    token: Option<String>,
}

impl KeyValueAwsDynamoRuntimeConfigOptions {
    pub fn new(access_key: String, secret_key: String, token: Option<String>) -> Self {
        Self {
            access_key,
            secret_key,
            token,
        }
    }
}

impl ProvideCredentials for KeyValueAwsDynamoRuntimeConfigOptions {
    fn provide_credentials<'a>(
        &'a self,
    ) -> aws_credential_types::provider::future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        aws_credential_types::provider::future::ProvideCredentials::ready(Ok(Credentials::new(
            self.access_key.clone(),
            self.secret_key.clone(),
            self.token.clone(),
            None, // Optional expiration time
            "spin_custom_aws_provider",
        )))
    }
}

/// AWS Dynamo Key / Value enumeration for the possible authentication options
#[derive(Clone, Debug)]
pub enum KeyValueAwsDynamoAuthOptions {
    /// Runtime Config values indicates credentials have been specified directly
    RuntimeConfigValues(KeyValueAwsDynamoRuntimeConfigOptions),
    /// Environmental indicates that the environment variables of the process should be used to
    /// create the SDK Config for the Dynamo client. This will use the AWS Rust SDK's
    /// aws_config::load_defaults to derive credentials based on what environment variables
    /// have been set.
    ///
    /// See https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-authentication.html for options.
    Environmental,
}

impl KeyValueAwsDynamo {
    pub fn new(
        region: String,
        table: String,
        auth_options: KeyValueAwsDynamoAuthOptions,
    ) -> Result<Self> {
        let config = match auth_options {
            KeyValueAwsDynamoAuthOptions::RuntimeConfigValues(config) => SdkConfig::builder()
                .credentials_provider(SharedCredentialsProvider::new(config))
                .region(Region::new(region))
                .behavior_version(BehaviorVersion::latest())
                .build(),
            KeyValueAwsDynamoAuthOptions::Environmental => {
                // aws_config::load_defaults(BehaviorVersion::latest()).await
                todo!("aws_config::load_defaults currently has no synchronous version");
            }
        };
        let client = Client::new(&config);

        Ok(Self { client, table })
    }
}

#[async_trait]
impl StoreManager for KeyValueAwsDynamo {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error> {
        Ok(Arc::new(AwsDynamoStore {
            _name: name.to_owned(),
            client: self.client.clone(),
            table: self.table.clone(),
        }))
    }

    fn is_defined(&self, _store_name: &str) -> bool {
        true
    }

    fn summary(&self, _store_name: &str) -> Option<String> {
        Some(format!(
            "AWS DynamoDB region: {:?}, table: {}",
            self.client
                .config()
                .region()
                .map(|region| region.to_string())
                .unwrap_or("NO REGION SET".into()),
            self.table
        ))
    }
}

struct AwsDynamoStore {
    _name: String,
    client: Client,
    table: String,
}

const PK: &str = "PK";
const VAL: &str = "val";

#[async_trait]
impl Store for AwsDynamoStore {
    #[instrument(name = "spin_key_value_aws.get", skip(self, key), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let item = self.get_item(key).await?;
        Ok(item)
    }

    #[instrument(name = "spin_key_value_aws.set", skip(self, key, value), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.client
            .put_item()
            .table_name(self.table.clone())
            .item(PK, AttributeValue::S(key.to_string()))
            .item(VAL, AttributeValue::B(Blob::new(value)))
            .send()
            .await
            .map_err(log_error)?;
        Ok(())
    }

    #[instrument(name = "spin_key_value_aws.delete", skip(self, key), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn delete(&self, key: &str) -> Result<(), Error> {
        if self.exists(key).await? {
            self.client
                .delete_item()
                .table_name(self.table.clone())
                .key(PK, AttributeValue::S(key.to_string()))
                .send()
                .await
                .map_err(log_error)?;
        }
        Ok(())
    }

    #[instrument(name = "spin_key_value_aws.exists", skip(self, key), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn exists(&self, key: &str) -> Result<bool, Error> {
        Ok(self.get_item(key).await?.is_some())
    }

    #[instrument(name = "spin_key_value_aws.get_keys", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        self.get_keys().await
    }
}

impl AwsDynamoStore {
    async fn get_item(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let query = self
            .client
            .get_item()
            .table_name(self.table.clone())
            .key(PK, aws_sdk_dynamodb::types::AttributeValue::S(key.into()))
            .send()
            .await
            .map_err(log_error)?;

        Ok(query.item.and_then(|item| {
            if let Some(AttributeValue::B(val)) = item.get(VAL) {
                Some(val.clone().into_inner())
            } else {
                None
            }
        }))
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        let mut primary_keys = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let mut scan_builder = self
                .client
                .scan()
                .table_name(self.table.clone())
                .projection_expression(PK);

            if let Some(keys) = last_evaluated_key {
                for (key, val) in keys {
                    scan_builder = scan_builder.exclusive_start_key(key, val);
                }
            }

            let scan_output = scan_builder.send().await.map_err(log_error)?;

            if let Some(items) = scan_output.items {
                for item in items {
                    if let Some(AttributeValue::S(pk)) = item.get(PK) {
                        primary_keys.push(pk.clone());
                    }
                }
            }

            last_evaluated_key = scan_output.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(primary_keys)
    }
}
