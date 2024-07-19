use serde::Deserialize;
use spin_factor_key_value::MakeKeyValueStore;
use spin_key_value_azure::KeyValueAzureCosmos;

/// A key-value store that uses Azure Cosmos as the backend.
pub struct AzureKeyValueStore;

/// Runtime configuration for the Azure Cosmos key-value store.
#[derive(Deserialize)]
pub struct AzureCosmosKeyValueRuntimeConfig {
    /// The authorization token for the Azure Cosmos DB account.
    key: String,
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
        KeyValueAzureCosmos::new(runtime_config.key, runtime_config.account, runtime_config.database, runtime_config.container)
    }
}
