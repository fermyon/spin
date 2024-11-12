use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_dynamodb::{
    config::{ProvideCredentials, SharedCredentialsProvider},
    operation::{
        batch_get_item::BatchGetItemOutput, batch_write_item::BatchWriteItemOutput,
        get_item::GetItemOutput, update_item::UpdateItemOutput,
    },
    primitives::Blob,
    types::{
        AttributeValue, DeleteRequest, KeysAndAttributes, PutRequest, TransactWriteItem, Update,
        WriteRequest,
    },
    Client,
};
use spin_core::async_trait;
use spin_factor_key_value::{log_error, Cas, Error, Store, StoreManager, SwapError};

pub struct KeyValueAwsDynamo {
    region: String,
    // Needs to be cloned when getting a store
    table: Arc<String>,
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
            region,
            table: Arc::new(table),
            client: async_once_cell::Lazy::from_future(client_fut),
        })
    }
}

#[async_trait]
impl StoreManager for KeyValueAwsDynamo {
    async fn get(&self, _name: &str) -> Result<Arc<dyn Store>, Error> {
        Ok(Arc::new(AwsDynamoStore {
            client: self.client.get_unpin().await.clone(),
            table: self.table.clone(),
        }))
    }

    fn is_defined(&self, _store_name: &str) -> bool {
        true
    }

    fn summary(&self, _store_name: &str) -> Option<String> {
        Some(format!(
            "AWS DynamoDB region: {}, table: {}",
            self.region, self.table
        ))
    }
}

struct AwsDynamoStore {
    // Client wraps an Arc so should be low cost to clone
    client: Client,
    table: Arc<String>,
}

struct CompareAndSwap {
    key: String,
    client: Client,
    table: Arc<String>,
    bucket_rep: u32,
    has_lock: Mutex<bool>,
}

/// Primary key in DynamoDB items used for querying items
const PK: &str = "PK";
/// Value key in DynamoDB items storing item value as binary
const VAL: &str = "val";
/// Lock key in DynamoDB items used for atomic operations
const LOCK: &str = "lock";

#[async_trait]
impl Store for AwsDynamoStore {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let response = self
            .client
            .get_item()
            .table_name(self.table.as_str())
            .key(
                PK,
                aws_sdk_dynamodb::types::AttributeValue::S(key.to_string()),
            )
            .projection_expression(VAL)
            .send()
            .await
            .map_err(log_error)?;

        let item = response.item.and_then(|mut item| {
            if let Some(AttributeValue::B(val)) = item.remove(VAL) {
                Some(val.into_inner())
            } else {
                None
            }
        });

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
        let GetItemOutput { item, .. } = self
            .client
            .get_item()
            .table_name(self.table.as_str())
            .key(
                PK,
                aws_sdk_dynamodb::types::AttributeValue::S(key.to_string()),
            )
            .projection_expression(PK)
            .send()
            .await
            .map_err(log_error)?;

        Ok(item.map(|item| item.contains_key(PK)).unwrap_or(false))
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        self.get_keys().await
    }

    async fn get_many(&self, keys: Vec<String>) -> Result<Vec<(String, Option<Vec<u8>>)>, Error> {
        let mut results = Vec::with_capacity(keys.len());

        let mut keys_and_attributes_builder =
            KeysAndAttributes::builder().projection_expression(format!("{PK},{VAL}"));
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

            request_items = unprocessed_keys.filter(|unprocessed| !unprocessed.is_empty());
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
            let BatchWriteItemOutput {
                unprocessed_items, ..
            } = self
                .client
                .batch_write_item()
                .set_request_items(request_items)
                .send()
                .await
                .map_err(log_error)?;

            request_items = unprocessed_items.filter(|unprocessed| !unprocessed.is_empty());
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
            let BatchWriteItemOutput {
                unprocessed_items, ..
            } = self
                .client
                .batch_write_item()
                .set_request_items(request_items)
                .send()
                .await
                .map_err(log_error)?;

            request_items = unprocessed_items.filter(|unprocessed| !unprocessed.is_empty());
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
            has_lock: Mutex::new(false),
            bucket_rep,
        }))
    }
}

#[async_trait]
impl Cas for CompareAndSwap {
    async fn current(&self) -> Result<Option<Vec<u8>>, Error> {
        let UpdateItemOutput { attributes, .. } = self
            .client
            .update_item()
            .table_name(self.table.as_str())
            .key(PK, AttributeValue::S(self.key.clone()))
            .update_expression("SET #lock=:lock")
            .expression_attribute_names("#lock", LOCK)
            .expression_attribute_values(":lock", AttributeValue::Null(true))
            .condition_expression("attribute_not_exists (#lock)")
            .return_values(aws_sdk_dynamodb::types::ReturnValue::AllNew)
            .send()
            .await
            .map_err(log_error)?;

        self.has_lock.lock().unwrap().clone_from(&true);

        match attributes {
            Some(mut item) => match item.remove(VAL) {
                Some(AttributeValue::B(val)) => Ok(Some(val.into_inner())),
                _ => Ok(None),
            },
            None => Ok(None),
        }
    }

    /// `swap` updates the value for the key using the version saved in the `current` function for
    /// optimistic concurrency.
    async fn swap(&self, value: Vec<u8>) -> Result<(), SwapError> {
        let mut update_item = Update::builder()
            .table_name(self.table.as_str())
            .key(PK, AttributeValue::S(self.key.clone()))
            .update_expression("SET #val=:val REMOVE #lock")
            .expression_attribute_names("#val", VAL)
            .expression_attribute_values(":val", AttributeValue::B(Blob::new(value)))
            .expression_attribute_names("#lock", LOCK);

        let has_lock = *self.has_lock.lock().unwrap();
        // Ensure exclusive access between fetching the current value of the item and swapping
        if has_lock {
            update_item = update_item.condition_expression("attribute_exists (#lock)");
        }

        // TransactWriteItems fails if concurrent writes are in progress on an item, so even without locking, we get atomicity in overwriting
        self.client
            .transact_write_items()
            .transact_items(
                TransactWriteItem::builder()
                    .update(
                        update_item
                            .build()
                            .map_err(|e| SwapError::Other(format!("{e:?}")))?,
                    )
                    .build(),
            )
            .send()
            .await
            .map_err(|e| SwapError::CasFailed(format!("{e:?}")))?;

        Ok(())
    }

    async fn bucket_rep(&self) -> u32 {
        self.bucket_rep
    }

    async fn key(&self) -> String {
        self.key.clone()
    }
}

impl AwsDynamoStore {
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
