use async_trait::async_trait;
use serde::Serialize;
use url::Url;
use chrono::{DateTime, Utc};
use serde_json::Value;
use erased_serde::Serialize as ErasedSerialize;

#[derive(Debug, Clone, Serialize)]
pub struct StorageItem<T: Serialize> {
    pub url: Url,
    pub timestamp: DateTime<Utc>,
    pub data: T,
    pub metadata: Option<Value>,
}

pub trait StorageConfig: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    fn create_config(&self, collection_name: &str) -> Box<dyn StorageConfig>;
    
    async fn store_serialized(
        &self, 
        item: StorageItem<Box<dyn ErasedSerialize + Send + Sync>>,
        config: &dyn StorageConfig,
    ) -> crate::ScraperResult<()>;
}

pub trait IntoStorageData {
    fn into_storage_data(self) -> Box<dyn ErasedSerialize + Send + Sync>;
}

impl<T: Serialize + Send + Sync + 'static> IntoStorageData for T {
    fn into_storage_data(self) -> Box<dyn ErasedSerialize + Send + Sync> {
        Box::new(self)
    }
}
