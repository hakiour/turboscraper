use super::base::{StorageBackend, StorageConfig, StorageError, StorageItem};
use anyhow::Error;
use async_trait::async_trait;
use erased_serde::Serialize as ErasedSerialize;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::error::Error as StdError;
use std::fmt;
use std::time::Duration;

#[derive(Debug)]
pub enum KafkaStorageError {
    Connection(rdkafka::error::KafkaError),
    Serialization(serde_json::Error),
    Operation(rdkafka::error::KafkaError),
}

impl fmt::Display for KafkaStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection(e) => write!(f, "Kafka connection error: {}", e),
            Self::Serialization(e) => write!(f, "Serialization error: {}", e),
            Self::Operation(e) => write!(f, "Kafka operation error: {}", e),
        }
    }
}

impl StdError for KafkaStorageError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Connection(e) => Some(e),
            Self::Serialization(e) => Some(e),
            Self::Operation(e) => Some(e),
        }
    }
}

#[derive(Clone)]
pub struct KafkaStorage {
    producer: FutureProducer,
}

impl KafkaStorage {
    pub fn new(brokers: &str, client_id: &str) -> Result<Self, Error> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("client.id", client_id)
            .set("message.timeout.ms", "65000")
            .create()
            .map_err(KafkaStorageError::Connection)?;

        Ok(Self { producer })
    }
}

#[derive(Debug, Clone)]
pub struct KafkaConfig {
    pub topic: String,
}

impl StorageConfig for KafkaConfig {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn StorageConfig> {
        Box::new(self.clone())
    }

    fn destination(&self) -> &str {
        &self.topic
    }
}

impl From<rdkafka::error::KafkaError> for StorageError {
    fn from(error: rdkafka::error::KafkaError) -> Self {
        StorageError::OperationError(error.to_string())
    }
}

#[async_trait]
impl StorageBackend for KafkaStorage {
    fn create_config(&self, topic: &str) -> Box<dyn StorageConfig> {
        Box::new(KafkaConfig {
            topic: topic.to_string(),
        })
    }

    async fn store_serialized(
        &self,
        item: StorageItem<Box<dyn ErasedSerialize + Send + Sync>>,
        config: &dyn StorageConfig,
    ) -> Result<(), StorageError> {
        let config = config
            .as_any()
            .downcast_ref::<KafkaConfig>()
            .expect("Invalid config type");

        let payload = serde_json::json!({
            "url": item.url.to_string(),
            "timestamp": item.timestamp,
            "data": item.data,
            "metadata": item.metadata,
            "id": item.id,
        });

        let key = item.id;
        let value = serde_json::to_string(&payload)?;

        self.producer
            .send(
                FutureRecord::to(config.destination())
                    .key(&key)
                    .payload(&value),
                Duration::from_secs(5),
            )
            .await
            .map_err(|(err, _)| StorageError::OperationError(err.to_string()))?;

        Ok(())
    }
}
