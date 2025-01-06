#[cfg(feature = "mongodb")]
use super::MongoStorage;
use super::{base::StorageError, DiskStorage, StorageBackend, StorageConfig, StorageItem};
use anyhow::Error;
use async_trait::async_trait;
use erased_serde::Serialize as ErasedSerialize;

pub enum StorageType {
    Disk {
        path: String,
    },
    #[cfg(feature = "mongodb")]
    Mongo {
        connection_string: String,
        database: String,
    },
}

#[derive(Clone)]
pub enum Storage {
    Disk(Box<DiskStorage>),
    #[cfg(feature = "mongodb")]
    Mongo(Box<MongoStorage>),
}

#[async_trait]
impl StorageBackend for Storage {
    fn create_config(&self, destination: &str) -> Box<dyn StorageConfig> {
        match self {
            Storage::Disk(storage) => storage.create_config(destination),
            #[cfg(feature = "mongodb")]
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
            #[cfg(feature = "mongodb")]
            Storage::Mongo(storage) => storage.store_serialized(item, config).await,
        }
    }
}

pub async fn create_storage(storage_type: StorageType) -> Result<Storage, Error> {
    match storage_type {
        StorageType::Disk { path } => Ok(Storage::Disk(Box::new(DiskStorage::new(path).unwrap()))),
        #[cfg(feature = "mongodb")]
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
