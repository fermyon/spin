use std::sync::Arc;

use anyhow::Result;
use azure_data_cosmos::{
    prelude::{AuthorizationToken, CollectionClient, CosmosClient, Query},
    CosmosEntity,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use spin_core::async_trait;
use spin_key_value::{log_error, Error, Store, StoreManager};

pub struct KeyValueAzureCosmos {
    client: CollectionClient,
}

impl KeyValueAzureCosmos {
    pub fn new(key: String, account: String, database: String, container: String) -> Result<Self> {
        let token = AuthorizationToken::primary_from_base64(&key).map_err(log_error)?;
        let cosmos_client = CosmosClient::new(account, token);
        let database_client = cosmos_client.database_client(database);
        let client = database_client.collection_client(container);

        Ok(Self { client })
    }
}

#[async_trait]
impl StoreManager for KeyValueAzureCosmos {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error> {
        Ok(Arc::new(AzureCosmosStore {
            _name: name.to_owned(),
            client: self.client.clone(),
        }))
    }
}

struct AzureCosmosStore {
    _name: String,
    client: CollectionClient,
}

#[async_trait]
impl Store for AzureCosmosStore {
    async fn get(&self, key: &str) -> Result<Vec<u8>, Error> {
        let pair = self.get_pair(key).await?;
        Ok(pair.value)
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        let pair = Pair {
            id: key.to_string(),
            value: value.to_vec(),
        };
        self.client
            .create_document(pair)
            .is_upsert(true)
            .await
            .map_err(log_error)?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), Error> {
        match self.exists(key).await {
            Ok(true) => {
                let document_client = self.client.document_client(key, &key).map_err(log_error)?;
                document_client.delete_document().await.map_err(log_error)?;
                Ok(())
            }
            // The Spin key-value interface mandates that no error should be raised if the pair did
            // not exist.
            Ok(false) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, Error> {
        match self.get_pair(key).await {
            Ok(_) => Ok(true),
            Err(Error::NoSuchKey) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        self.get_keys().await
    }
}

impl AzureCosmosStore {
    async fn get_pair(&self, key: &str) -> Result<Pair, Error> {
        let query = self
            .client
            .query_documents(Query::new(format!("SELECT * FROM c WHERE c.id='{}'", key)))
            .query_cross_partition(true)
            .max_item_count(1);

        // There can be no duplicated keys, so we create the stream and only take the first result.
        let mut stream = query.into_stream::<Pair>();
        let res = stream.next().await;
        match res {
            Some(r) => {
                let r = r.map_err(log_error)?;
                match r.results.first().cloned() {
                    Some((p, _)) => Ok(p),
                    None => Err(Error::NoSuchKey),
                }
            }
            None => Err(Error::NoSuchKey),
        }
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        let query = self
            .client
            .query_documents(Query::new("SELECT * FROM c".to_string()))
            .query_cross_partition(true);
        let mut res = Vec::new();

        let mut stream = query.into_stream::<Pair>();
        while let Some(resp) = stream.next().await {
            let resp = resp.map_err(log_error)?;
            for (pair, _) in resp.results {
                res.push(pair.id);
            }
        }

        Ok(res)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pair {
    // In Azure CosmosDB, the default partition key is "/id", and this implementation assumes that partition ID is not changed.
    pub id: String,
    pub value: Vec<u8>,
}

impl CosmosEntity for Pair {
    type Entity = String;

    fn partition_key(&self) -> Self::Entity {
        self.id.clone()
    }
}
