use crate::{http::HttpRequest, HttpResponse, ScraperResult};
use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;

use super::retry::RetryConfig;
use super::ScraperError;
use crate::core::retry::RetryCategory;
use crate::storage::{
    IntoStorageData, StorageBackend, StorageCategory, StorageItem, StorageManager,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum SpiderCallback {
    Bootstrap,       // For initial page
    ParseItem,       // For parsing detail pages (e.g., product pages)
    ParsePagination, // For handling pagination
    Custom(String),  // For custom parsing methods
}

#[derive(Debug)]
pub enum ParseResult {
    Continue(Vec<HttpRequest>),
    Skip,
    Stop,
    RetryWithSameContent(Box<HttpResponse>),
    RetryWithNewContent(Box<HttpRequest>), // Include the request to retry
}

#[derive(Debug)]
pub enum ParsedData {
    Item(serde_json::Value),
    Items(Vec<serde_json::Value>),
    Raw(String),
    Empty,
}

#[derive(Debug, Clone)]
pub struct SpiderResponse {
    pub response: HttpResponse,
    pub callback: SpiderCallback,
}

#[derive(Debug, Clone)]
pub struct SpiderConfig {
    pub max_depth: usize,
    pub max_concurrency: usize,
    pub retry_config: RetryConfig,
    pub headers: HashMap<String, String>,
    pub allow_url_revisit: bool,
}

impl Default for SpiderConfig {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_concurrency: 10,
            retry_config: RetryConfig::default(),
            headers: HashMap::new(),
            allow_url_revisit: false,
        }
    }
}

impl SpiderConfig {
    pub fn with_retry(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    pub fn with_headers(mut self, headers: Vec<(&str, &str)>) -> Self {
        for (key, value) in headers {
            self.headers.insert(key.to_string(), value.to_string());
        }
        self
    }

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.max_concurrency = concurrency;
        self
    }

    pub fn with_allow_url_revisit(mut self, allow: bool) -> Self {
        self.allow_url_revisit = allow;
        self
    }
}

#[async_trait]
pub trait Spider: Sized {
    fn name(&self) -> String;
    fn config(&self) -> &SpiderConfig;
    fn set_config(&mut self, config: SpiderConfig);
    fn start_requests(&self) -> Vec<HttpRequest>;

    /// Extract data from the response and determine the next actions to take.
    /// This is a synchronous operation that doesn't involve any I/O.
    fn parse(&self, response: &SpiderResponse) -> ScraperResult<(ParseResult, ParsedData)>;

    /// Persist the extracted data to the configured storage backend.
    /// This is an asynchronous operation that handles I/O.
    async fn persist_extracted_data(
        &self,
        data: ParsedData,
        response: &SpiderResponse,
    ) -> ScraperResult<()>;

    /// Main coordinator that handles the full extraction and persistence flow.
    async fn process_response(&self, response: &SpiderResponse) -> ScraperResult<ParseResult> {
        let (parse_result, parsed_data) = self.parse(response)?;
        self.persist_extracted_data(parsed_data, response).await?;
        Ok(parse_result)
    }

    fn get_initial_callback(&self) -> SpiderCallback {
        SpiderCallback::Bootstrap
    }

    fn allowed_domains(&self) -> Option<Vec<String>> {
        None
    }

    fn with_config(mut self, config: SpiderConfig) -> Self {
        self.set_config(config);
        self
    }

    /// Handle maximum retries reached error
    /// This is called when a request has reached its maximum retry attempts
    /// Implementations can choose to store the error, log it, or take other actions
    async fn handle_max_retries(
        &self,
        category: RetryCategory,
        request: Box<HttpRequest>,
    ) -> ScraperResult<()>;

    fn storage_manager(&self) -> &StorageManager;

    async fn store_data<T: IntoStorageData + Send + Sync + Serialize>(
        &self,
        item: StorageItem<T>,
        category: StorageCategory,
        request: Box<HttpRequest>,
    ) -> ScraperResult<()> {
        let manager = self.storage_manager();
        let (storage, config) = manager.get_storage(&category);

        let item = StorageItem {
            url: item.url,
            timestamp: item.timestamp,
            data: item.data.into_storage_data(),
            metadata: item.metadata,
            id: item.id,
        };

        storage
            .store_serialized(item, &**config)
            .await
            .map_err(|e| (ScraperError::StorageError(e), request))
    }
}
