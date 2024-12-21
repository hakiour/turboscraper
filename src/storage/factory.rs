use super::{DiskStorage, MongoStorage, StorageBackend};
use crate::ScraperResult;

pub enum StorageType {
    Disk { path: String },
    Mongo { uri: String, database: String },
}

pub async fn create_storage(storage_type: StorageType) -> ScraperResult<Box<dyn StorageBackend>> {
    match storage_type {
        StorageType::Disk { path } => {
            Ok(Box::new(DiskStorage::new(path)?))
        },
        StorageType::Mongo { uri, database } => {
            Ok(Box::new(MongoStorage::new(&uri, &database).await?))
        }
    }
} 