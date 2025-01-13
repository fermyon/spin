mod store;

use serde::Deserialize;
use spin_factor_key_value::runtime_config::spin::MakeKeyValueStore;

pub use store::{
    KeyValueAzureCosmos, KeyValueAzureCosmosAuthOptions, KeyValueAzureCosmosRuntimeConfigOptions,
};

/// A key-value store that uses Azure Cosmos as the backend.
pub struct AzureKeyValueStore {
    app_id: Option<String>,
}

impl AzureKeyValueStore {
    /// Creates a new `AzureKeyValueStore`.
    ///
    /// When `app_id` is provided, the store will a partition key of `$app_id/$store_name`,
    /// otherwise the partition key will be `id`.
    pub fn new(app_id: Option<String>) -> Self {
        Self { app_id }
    }
}

/// Runtime configuration for the Azure Cosmos key-value store.
#[derive(Deserialize)]
pub struct AzureCosmosKeyValueRuntimeConfig {
    /// The authorization token for the Azure Cosmos DB account.
    key: Option<String>,
    /// The Azure Cosmos DB account name.
    account: String,
    /// The Azure Cosmos DB database.
    database: String,
    /// The Azure Cosmos DB container where data is stored.
    /// The CosmosDB container must be created with the default partition key, /id
    container: String,
}

impl MakeKeyValueStore for AzureKeyValueStore {
    const RUNTIME_CONFIG_TYPE: &'static str = "azure_cosmos";

    type RuntimeConfig = AzureCosmosKeyValueRuntimeConfig;

    type StoreManager = KeyValueAzureCosmos;

    fn make_store(
        &self,
        runtime_config: Self::RuntimeConfig,
    ) -> anyhow::Result<Self::StoreManager> {
        let auth_options = match runtime_config.key {
            Some(key) => KeyValueAzureCosmosAuthOptions::RuntimeConfigValues(
                KeyValueAzureCosmosRuntimeConfigOptions::new(key),
            ),
            None => KeyValueAzureCosmosAuthOptions::Environmental,
        };
        KeyValueAzureCosmos::new(
            runtime_config.account,
            runtime_config.database,
            runtime_config.container,
            auth_options,
            self.app_id.clone(),
        )
    }
}
