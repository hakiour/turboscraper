use super::{
    base::StorageError, DiskStorage, MongoStorage, StorageBackend, StorageConfig, StorageItem,
};
use anyhow::Error;
use async_trait::async_trait;
use erased_serde::Serialize as ErasedSerialize;

pub enum StorageType {
    Disk {
        path: String,
    },
    Mongo {
        connection_string: String,
        database: String,
    },
}

#[derive(Clone)]
pub enum Storage {
    Disk(Box<DiskStorage>),
    Mongo(Box<MongoStorage>),
}

#[async_trait]
impl StorageBackend for Storage {
    fn create_config(&self, destination: &str) -> Box<dyn StorageConfig> {
        match self {
            Storage::Disk(storage) => storage.create_config(destination),
            Storage::Mongo(storage) => storage.create_config(destination),
        }
    }

    async fn store_serialized(
        &self,
        item: StorageItem<Box<dyn ErasedSerialize + Send + Sync>>,
        config: &dyn StorageConfig,
    ) -> Result<(), StorageError> {
        match self {
            Storage::Disk(storage) => storage.store_serialized(item, config).await,
            Storage::Mongo(storage) => storage.store_serialized(item, config).await,
        }
    }
}

pub async fn create_storage(storage_type: StorageType) -> Result<Storage, Error> {
    match storage_type {
        StorageType::Disk { path } => Ok(Storage::Disk(Box::new(DiskStorage::new(path).unwrap()))),
        StorageType::Mongo {
            connection_string,
            database,
        } => Ok(Storage::Mongo(Box::new(
            MongoStorage::new(&connection_string, &database)
                .await
                .unwrap(),
        ))),
    }
}
