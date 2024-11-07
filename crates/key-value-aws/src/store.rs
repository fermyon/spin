use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_dynamodb::{
    config::{ProvideCredentials, SharedCredentialsProvider},
    operation::{batch_get_item::BatchGetItemOutput, get_item::GetItemOutput},
    primitives::Blob,
    types::{AttributeValue, DeleteRequest, KeysAndAttributes, PutRequest, WriteRequest},
    Client,
};
use spin_core::async_trait;
use spin_factor_key_value::{log_error, Cas, Error, Store, StoreManager, SwapError};

pub struct KeyValueAwsDynamo {
    table: Arc<String>,
    region: Arc<String>,
    client: async_once_cell::Lazy<
        Client,
        std::pin::Pin<Box<dyn std::future::Future<Output = Client> + Send>>,
    >,
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
        let region_clone = region.clone();
        let client_fut = Box::pin(async move {
            let sdk_config = match auth_options {
                KeyValueAwsDynamoAuthOptions::RuntimeConfigValues(config) => SdkConfig::builder()
                    .credentials_provider(SharedCredentialsProvider::new(config))
                    .region(Region::new(region_clone))
                    .behavior_version(BehaviorVersion::latest())
                    .build(),
                KeyValueAwsDynamoAuthOptions::Environmental => {
                    aws_config::load_defaults(BehaviorVersion::latest()).await
                }
            };
            Client::new(&sdk_config)
        });

        Ok(Self {
            table: Arc::new(table),
            region: Arc::new(region),
            client: async_once_cell::Lazy::from_future(client_fut),
        })
    }
}

#[async_trait]
impl StoreManager for KeyValueAwsDynamo {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error> {
        Ok(Arc::new(AwsDynamoStore {
            _name: name.to_owned(),
            client: self.client.get_unpin().await.clone(),
            table: self.table.clone(),
        }))
    }

    fn is_defined(&self, _store_name: &str) -> bool {
        true
    }

    fn summary(&self, _store_name: &str) -> Option<String> {
        Some(format!(
            "AWS DynamoDB region: {:?}, table: {}",
            self.region, self.table
        ))
    }
}

struct AwsDynamoStore {
    _name: String,
    client: Client,
    table: Arc<String>,
}

struct CompareAndSwap {
    key: String,
    client: Client,
    table: Arc<String>,
    bucket_rep: u32,
    etag: Mutex<Option<String>>,
}

/// Primary key in DynamoDB items used for querying items
const PK: &str = "PK";
/// Value key in DynamoDB items storing item value as binary
const VAL: &str = "val";
/// Version key in DynamoDB items used for optimistic locking
const VER: &str = "ver";

#[async_trait]
impl Store for AwsDynamoStore {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let item = self.get_item(key).await?;
        Ok(item)
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.client
            .put_item()
            .table_name(self.table.as_str())
            .item(PK, AttributeValue::S(key.to_string()))
            .item(VAL, AttributeValue::B(Blob::new(value)))
            .send()
            .await
            .map_err(log_error)?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), Error> {
        if self.exists(key).await? {
            self.client
                .delete_item()
                .table_name(self.table.as_str())
                .key(PK, AttributeValue::S(key.to_string()))
                .send()
                .await
                .map_err(log_error)?;
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, Error> {
        Ok(self.get_item(key).await?.is_some())
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        self.get_keys().await
    }

    async fn get_many(&self, keys: Vec<String>) -> Result<Vec<(String, Option<Vec<u8>>)>, Error> {
        let mut results = Vec::with_capacity(keys.len());

        let mut keys_and_attributes_builder = KeysAndAttributes::builder();
        for key in keys {
            keys_and_attributes_builder = keys_and_attributes_builder.keys(HashMap::from_iter([(
                PK.to_owned(),
                AttributeValue::S(key),
            )]))
        }
        let mut request_items = Some(HashMap::from_iter([(
            self.table.to_string(),
            keys_and_attributes_builder.build().map_err(log_error)?,
        )]));

        while request_items.is_some() {
            let BatchGetItemOutput {
                responses: Some(mut responses),
                unprocessed_keys,
                ..
            } = self
                .client
                .batch_get_item()
                .set_request_items(request_items)
                .send()
                .await
                .map_err(log_error)?
            else {
                return Err(Error::Other("No results".into()));
            };

            if let Some(items) = responses.remove(self.table.as_str()) {
                for mut item in items {
                    let Some(AttributeValue::S(pk)) = item.remove(PK) else {
                        return Err(Error::Other(
                            "Could not find 'PK' key on DynamoDB item".into(),
                        ));
                    };
                    let Some(AttributeValue::B(val)) = item.remove(VAL) else {
                        return Err(Error::Other(
                            "Could not find 'val' key on DynamoDB item".into(),
                        ));
                    };

                    results.push((pk, Some(val.into_inner())));
                }
            }

            request_items = unprocessed_keys;
        }

        Ok(results)
    }

    async fn set_many(&self, key_values: Vec<(String, Vec<u8>)>) -> Result<(), Error> {
        let mut data = Vec::with_capacity(key_values.len());
        for (key, val) in key_values {
            data.push(
                WriteRequest::builder()
                    .put_request(
                        PutRequest::builder()
                            .item(PK, AttributeValue::S(key))
                            .item(VAL, AttributeValue::B(Blob::new(val)))
                            .build()
                            .map_err(log_error)?,
                    )
                    .build(),
            )
        }

        let mut request_items = Some(HashMap::from_iter([(self.table.to_string(), data)]));

        while request_items.is_some() {
            let results = self
                .client
                .batch_write_item()
                .set_request_items(request_items)
                .send()
                .await
                .map_err(log_error)?;

            request_items = results.unprocessed_items;
        }

        Ok(())
    }

    async fn delete_many(&self, keys: Vec<String>) -> Result<(), Error> {
        let mut data = Vec::with_capacity(keys.len());
        for key in keys {
            data.push(
                WriteRequest::builder()
                    .delete_request(
                        DeleteRequest::builder()
                            .key(PK, AttributeValue::S(key))
                            .build()
                            .map_err(log_error)?,
                    )
                    .build(),
            )
        }

        let mut request_items = Some(HashMap::from_iter([(self.table.to_string(), data)]));

        while request_items.is_some() {
            let results = self
                .client
                .batch_write_item()
                .set_request_items(request_items)
                .send()
                .await
                .map_err(log_error)?;

            request_items = results.unprocessed_items;
        }

        Ok(())
    }

    async fn increment(&self, key: String, delta: i64) -> Result<i64, Error> {
        let result = self
            .client
            .update_item()
            .table_name(self.table.as_str())
            .key(PK, AttributeValue::S(key))
            .update_expression("ADD #val :delta")
            .expression_attribute_names("#val", VAL)
            .expression_attribute_values(":delta", AttributeValue::N(delta.to_string()))
            .return_values(aws_sdk_dynamodb::types::ReturnValue::UpdatedNew)
            .send()
            .await
            .map_err(log_error)?;

        if let Some(updated_attributes) = result.attributes {
            if let Some(AttributeValue::N(new_value)) = updated_attributes.get(VAL) {
                return Ok(new_value.parse::<i64>().map_err(log_error))?;
            }
        }

        Err(Error::Other("Failed to increment value".into()))
    }

    async fn new_compare_and_swap(
        &self,
        bucket_rep: u32,
        key: &str,
    ) -> Result<Arc<dyn spin_factor_key_value::Cas>, Error> {
        Ok(Arc::new(CompareAndSwap {
            key: key.to_string(),
            client: self.client.clone(),
            table: self.table.clone(),
            etag: Mutex::new(None),
            bucket_rep,
        }))
    }
}

#[async_trait]
impl Cas for CompareAndSwap {
    async fn current(&self) -> Result<Option<Vec<u8>>, Error> {
        let GetItemOutput {
            item: Some(mut current_item),
            ..
        } = self
            .client
            .get_item()
            .table_name(self.table.as_str())
            .key(
                PK,
                aws_sdk_dynamodb::types::AttributeValue::S(self.key.clone()),
            )
            .send()
            .await
            .map_err(log_error)?
        else {
            return Ok(None);
        };

        if let Some(AttributeValue::B(val)) = current_item.remove(VAL) {
            let version = if let Some(AttributeValue::N(ver)) = current_item.remove(VER) {
                Some(ver)
            } else {
                Some(String::from("0"))
            };
            self.etag.lock().unwrap().clone_from(&version);
            Ok(Some(val.into_inner()))
        } else {
            Ok(None)
        }
    }

    /// `swap` updates the value for the key using the etag saved in the `current` function for
    /// optimistic concurrency.
    async fn swap(&self, value: Vec<u8>) -> Result<(), SwapError> {
        let mut update_item = self
            .client
            .update_item()
            .table_name(self.table.as_str())
            .key(PK, AttributeValue::S(self.key.clone()))
            .update_expression("SET #val=:val, ADD #ver :increment")
            .expression_attribute_names("#val", VAL)
            .expression_attribute_names("#ver", VER)
            .expression_attribute_values(":val", AttributeValue::B(Blob::new(value)))
            .expression_attribute_values(":increment", AttributeValue::N("1".to_owned()))
            .return_values(aws_sdk_dynamodb::types::ReturnValue::None);

        let current_version = self.etag.lock().unwrap().clone();
        match current_version {
            // Existing item with no version key, update under condition that version key still does not exist in Dynamo when operation is executed
            Some(version) if version == "0" => {
                update_item = update_item.condition_expression("attribute_not_exists(#ver)");
            }
            // Existing item with version key, update under condition that version in Dynamo matches stored version
            Some(version) => {
                update_item = update_item
                    .condition_expression("#ver = :ver")
                    .expression_attribute_values(":ver", AttributeValue::N(version));
            }
            // Assume new item, insert under condition that item does not already exist
            None => {
                update_item = update_item
                    .condition_expression("attribute_not_exists(#pk)")
                    .expression_attribute_names("#pk", PK);
            }
        }

        update_item
            .send()
            .await
            .map(|_| ())
            .map_err(|e| SwapError::CasFailed(format!("{e:?}")))
    }

    async fn bucket_rep(&self) -> u32 {
        self.bucket_rep
    }

    async fn key(&self) -> String {
        self.key.clone()
    }
}

impl AwsDynamoStore {
    async fn get_item(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let response = self
            .client
            .get_item()
            .table_name(self.table.as_str())
            .key(
                PK,
                aws_sdk_dynamodb::types::AttributeValue::S(key.to_string()),
            )
            .send()
            .await
            .map_err(log_error)?;

        let val = response.item.and_then(|mut item| {
            if let Some(AttributeValue::B(val)) = item.remove(VAL) {
                Some(val.into_inner())
            } else {
                None
            }
        });

        Ok(val)
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        let mut primary_keys = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let mut scan_builder = self
                .client
                .scan()
                .table_name(self.table.as_str())
                .projection_expression(PK);

            if let Some(keys) = last_evaluated_key {
                for (key, val) in keys {
                    scan_builder = scan_builder.exclusive_start_key(key, val);
                }
            }

            let scan_output = scan_builder.send().await.map_err(log_error)?;

            if let Some(items) = scan_output.items {
                for mut item in items {
                    if let Some(AttributeValue::S(pk)) = item.remove(PK) {
                        primary_keys.push(pk);
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
