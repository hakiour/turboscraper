use super::base::{StorageBackend, StorageConfig, StorageItem};
use crate::ScraperResult;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;
use erased_serde::Serialize as ErasedSerialize;

pub struct DiskStorage {
    base_path: PathBuf,
}

impl DiskStorage {
    pub fn new<P: AsRef<Path>>(base_path: P) -> ScraperResult<Self> {
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
    ) -> ScraperResult<()> {
        let config = config.as_any()
            .downcast_ref::<DiskConfig>()
            .expect("Invalid config type");

        let mut path = self.base_path.clone();
        if let Some(ref subfolder) = config.subfolder {
            path = path.join(subfolder);
        }

        let timestamp = item.timestamp.format("%Y%m%d_%H%M%S");
        let host = item.url.host_str().unwrap_or("unknown");
        let prefix = config.filename_prefix.as_deref().unwrap_or("");
        let filename = format!("{}{}_{}.json", prefix, timestamp, Uuid::now_v7());
        
        let final_path = path.join(host).join(filename);
        fs::create_dir_all(final_path.parent().unwrap())?;
        
        let json = serde_json::json!({
            "url": item.url.to_string(),
            "timestamp": item.timestamp,
            "data": item.data,
            "metadata": item.metadata,
        });

        fs::write(final_path, serde_json::to_string_pretty(&json)?)?;
        Ok(())
    }
} 