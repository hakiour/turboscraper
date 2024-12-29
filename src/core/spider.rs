use crate::{http::HttpRequest, HttpResponse, ScraperResult};
use async_trait::async_trait;
use std::collections::HashMap;
use url::Url;

use super::retry::RetryConfig;

#[derive(Debug, Clone, PartialEq)]
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

    async fn parse(
        &self,
        response: SpiderResponse,
        url: Url,
        depth: usize,
    ) -> ScraperResult<ParseResult>;

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
}
