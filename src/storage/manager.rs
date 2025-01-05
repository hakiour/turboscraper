use super::{base::StorageBackend, factory::Storage, StorageCategory, StorageConfig};
use crate::ScraperResult;
use std::collections::HashMap;

#[derive(Clone)]
pub struct StorageManager {
    storages: HashMap<StorageCategory, (Storage, Box<dyn StorageConfig>)>,
    default_storage: StorageCategory,
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            storages: HashMap::new(),
            default_storage: StorageCategory::default(),
        }
    }

    pub fn register_storage(
        mut self,
        category: StorageCategory,
        storage: Storage,
        destination: &str,
    ) -> Self {
        let config = storage.create_config(destination);
        self.storages.insert(category.clone(), (storage, config));

        self
    }

    pub fn set_default_storage(mut self, category: StorageCategory) -> ScraperResult<Self> {
        self.default_storage = category;
        Ok(self)
    }

    pub fn get_storage(&self, category: &StorageCategory) -> &(Storage, Box<dyn StorageConfig>) {
        self.storages
            .get(category)
            .unwrap_or_else(|| self.get_default_storage())
    }

    pub fn get_default_storage(&self) -> &(Storage, Box<dyn StorageConfig>) {
        self.storages.get(&self.default_storage).unwrap()
    }
}
