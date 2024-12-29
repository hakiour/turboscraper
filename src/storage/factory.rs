use super::{DiskStorage, MongoStorage, StorageBackend};
use anyhow::Error;

pub enum StorageType {
    Disk {
        path: String,
    },
    Mongo {
        connection_string: String,
        database: String,
    },
}

pub async fn create_storage(storage_type: StorageType) -> Result<Box<dyn StorageBackend>, Error> {
    match storage_type {
        StorageType::Disk { path } => Ok(Box::new(DiskStorage::new(path).unwrap())),
        StorageType::Mongo {
            connection_string,
            database,
        } => Ok(Box::new(
            MongoStorage::new(&connection_string, &database)
                .await
                .unwrap(),
        )),
    }
}
