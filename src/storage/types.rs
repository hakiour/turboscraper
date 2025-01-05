use serde::Serialize;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Default)]
pub enum StorageCategory {
    #[default]
    Data, // For valid, processed data
    Error,          // For error logs and failed requests
    Raw,            // For raw responses
    Custom(String), // For any custom storage needs
}
