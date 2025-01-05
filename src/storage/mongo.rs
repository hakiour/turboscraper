use super::base::{StorageBackend, StorageConfig, StorageError, StorageItem};
use crate::ScraperError;
use anyhow::Error;
use async_trait::async_trait;
use erased_serde::Serialize as ErasedSerialize;
use mongodb::{bson::doc, error::Error as MongoError, Client};

// Unified error type for MongoDB operations
#[derive(Debug)]
pub enum MongoStorageError {
    Connection(MongoError),
    Serialization(mongodb::bson::ser::Error),
    Operation(MongoError),
}

#[derive(Clone)]
pub struct MongoStorage {
    database_name: String,
    client: Client,
}

impl MongoStorage {
    pub async fn new(connection_string: &str, database_name: &str) -> Result<Self, Error> {
        let client = Client::with_uri_str(connection_string)
            .await
            .map_err(MongoStorageError::Connection)
            .unwrap();

        Ok(Self {
            database_name: database_name.to_string(),
            client,
        })
    }

    async fn serialize_item(
        &self,
        item: StorageItem<Box<dyn ErasedSerialize + Send + Sync>>,
    ) -> Result<mongodb::bson::Document, MongoStorageError> {
        Ok(doc! {
            "url": item.url.to_string(),
            "timestamp": item.timestamp.to_rfc3339(),
            "data": mongodb::bson::to_bson(&item.data)
                .map_err(MongoStorageError::Serialization)?,
            "metadata": item.metadata
                .map(|m| mongodb::bson::to_bson(&m))
                .transpose()
                .map_err(MongoStorageError::Serialization)?
                .unwrap_or_default(),
        })
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

    fn clone_box(&self) -> Box<dyn StorageConfig> {
        Box::new(self.clone())
    }
}

impl From<MongoError> for StorageError {
    fn from(error: MongoError) -> Self {
        match *error.kind {
            mongodb::error::ErrorKind::Write(_) => StorageError::OperationError(error.to_string()),
            mongodb::error::ErrorKind::Command(_) => {
                StorageError::ConnectionError(error.to_string())
            }
            _ => StorageError::OperationError(error.to_string()),
        }
    }
}

impl From<mongodb::bson::ser::Error> for StorageError {
    fn from(error: mongodb::bson::ser::Error) -> Self {
        StorageError::SerializationError(error.to_string())
    }
}

impl From<MongoStorageError> for StorageError {
    fn from(error: MongoStorageError) -> Self {
        match error {
            MongoStorageError::Connection(e) => StorageError::ConnectionError(e.to_string()),
            MongoStorageError::Serialization(e) => StorageError::SerializationError(e.to_string()),
            MongoStorageError::Operation(e) => StorageError::OperationError(e.to_string()),
        }
    }
}

impl From<MongoStorageError> for ScraperError {
    fn from(error: MongoStorageError) -> Self {
        let error_msg = match error {
            MongoStorageError::Connection(e) => format!("MongoDB connection error: {}", e),
            MongoStorageError::Serialization(e) => format!("BSON serialization error: {}", e),
            MongoStorageError::Operation(e) => format!("MongoDB operation error: {}", e),
        };
        ScraperError::StorageError(StorageError::OperationError(error_msg))
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
    ) -> Result<(), StorageError> {
        let config = config
            .as_any()
            .downcast_ref::<MongoConfig>()
            .expect("Invalid config type");

        let doc = self
            .serialize_item(item)
            .await
            .map_err(StorageError::from)?;

        self.client
            .database(&self.database_name)
            .collection(&config.collection)
            .insert_one(doc)
            .await
            .map_err(StorageError::from)?;

        Ok(())
    }
}
