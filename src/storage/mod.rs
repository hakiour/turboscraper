pub mod base;
pub mod disk;
pub mod factory;
pub mod manager;

#[cfg(feature = "mongodb")]
pub mod mongo;
pub mod types;

pub use base::{IntoStorageData, StorageBackend, StorageConfig, StorageItem};
pub use disk::DiskStorage;
pub use factory::{create_storage, Storage, StorageType};
pub use manager::StorageManager;
#[cfg(feature = "mongodb")]
pub use mongo::MongoStorage;
pub use types::StorageCategory;
