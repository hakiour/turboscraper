use super::base::{StorageBackend, StorageConfig, StorageItem};
use crate::{ScraperError, ScraperResult};
use async_trait::async_trait;
use mongodb::{Client, Database, bson::doc};
use erased_serde::Serialize as ErasedSerialize;
pub struct MongoStorage {
    db: Database,
}

impl MongoStorage {
    pub async fn new(connection_string: &str, database: &str) -> ScraperResult<Self> {
        let client = Client::with_uri_str(connection_string).await?;
        let db = client.database(database);
        Ok(Self { db })
    }
}

#[derive(Debug, Clone)]
pub struct MongoConfig {
    pub collection: String,
}

impl StorageConfig for MongoConfig {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl StorageBackend for MongoStorage {
    fn create_config(&self, collection_name: &str) -> Box<dyn StorageConfig> {
        Box::new(MongoConfig {
            collection: collection_name.to_string(),
        })
    }

    async fn store_serialized(
        &self, 
        item: StorageItem<Box<dyn ErasedSerialize + Send + Sync>>,
        config: &dyn StorageConfig,
    ) -> crate::ScraperResult<()> {
        let config = config.as_any()
            .downcast_ref::<MongoConfig>()
            .expect("Invalid config type");

        let collection = self.db.collection(&config.collection);
        let doc = doc! {
            "url": item.url.to_string(),
            "timestamp": item.timestamp.to_rfc3339(),
            "data": mongodb::bson::to_bson(&item.data)?,
            "metadata": item.metadata.map(|m| mongodb::bson::to_bson(&m).unwrap()).unwrap_or_default(),
        };
        collection.insert_one(doc, None).await?;
        Ok(())
    }
}

impl From<mongodb::bson::ser::Error> for ScraperError {
    fn from(err: mongodb::bson::ser::Error) -> Self {
        ScraperError::StorageError(err.to_string())
    }
}

impl From<mongodb::error::Error> for ScraperError {
    fn from(err: mongodb::error::Error) -> Self {
        ScraperError::StorageError(err.to_string())
    }
}
