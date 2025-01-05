use async_trait::async_trait;
use chrono::{DateTime, Utc};
use erased_serde::Serialize as ErasedSerialize;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Serialize)]
pub struct StorageItem<T: Serialize> {
    pub url: Url,
    pub timestamp: DateTime<Utc>,
    pub data: T,
    pub metadata: Option<Value>,
    pub id: String,
}

pub trait StorageConfig: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn clone_box(&self) -> Box<dyn StorageConfig>;
    fn destination(&self) -> &str;
}

impl Clone for Box<dyn StorageConfig> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone, Error)]
pub enum StorageError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Operation error: {0}")]
    OperationError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    fn create_config(&self, collection_name: &str) -> Box<dyn StorageConfig>;

    async fn store_serialized(
        &self,
        item: StorageItem<Box<dyn ErasedSerialize + Send + Sync>>,
        config: &dyn StorageConfig,
    ) -> Result<(), StorageError>;
}

pub trait IntoStorageData {
    fn into_storage_data(self) -> Box<dyn ErasedSerialize + Send + Sync>;
}

impl<T: Serialize + Send + Sync + 'static> IntoStorageData for T {
    fn into_storage_data(self) -> Box<dyn ErasedSerialize + Send + Sync> {
        Box::new(self)
    }
}
