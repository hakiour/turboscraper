use super::base::{StorageBackend, StorageConfig, StorageError, StorageItem};
use anyhow::Error;
use async_trait::async_trait;
use erased_serde::Serialize as ErasedSerialize;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone)]
pub struct DiskStorage {
    base_path: PathBuf,
}

impl DiskStorage {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self, Error> {
        let base_path = base_path.as_ref().to_path_buf();
        fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }
}

#[derive(Debug, Clone)]
pub struct DiskConfig {
    pub subfolder: Option<String>,
    pub filename_prefix: Option<String>,
}

impl StorageConfig for DiskConfig {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn StorageConfig> {
        Box::new(self.clone())
    }
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        StorageError::OperationError(error.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(error: serde_json::Error) -> Self {
        StorageError::SerializationError(error.to_string())
    }
}

#[async_trait]
impl StorageBackend for DiskStorage {
    fn create_config(&self, collection_name: &str) -> Box<dyn StorageConfig> {
        Box::new(DiskConfig {
            subfolder: Some(collection_name.to_string()),
            filename_prefix: None,
        })
    }

    async fn store_serialized(
        &self,
        item: StorageItem<Box<dyn ErasedSerialize + Send + Sync>>,
        config: &dyn StorageConfig,
    ) -> Result<(), StorageError> {
        let config = config
            .as_any()
            .downcast_ref::<DiskConfig>()
            .expect("Invalid config type");

        let mut path = self.base_path.clone();
        if let Some(ref subfolder) = config.subfolder {
            path = path.join(subfolder);
        }

        let timestamp = item.timestamp.format("%Y%m%d_%H%M%S");
        let host = item.url.host_str().unwrap_or("unknown");
        let prefix = config.filename_prefix.as_deref().unwrap_or("");
        let id = item.id;
        let filename = format!("{}{}_{}_{}.json", prefix, timestamp, id, Uuid::now_v7());

        let final_path = path.join(host).join(filename);
        fs::create_dir_all(final_path.parent().unwrap())?;

        let json = serde_json::json!({
            "url": item.url.to_string(),
            "timestamp": item.timestamp,
            "data": item.data,
            "metadata": item.metadata,
            "id": id,
        });

        fs::write(final_path, serde_json::to_string_pretty(&json)?)?;
        Ok(())
    }
}
