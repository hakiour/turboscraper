use super::{Response, RetryConfig, Scraper};
use crate::errors::ScraperResult;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::sleep;
use url::Url;

#[derive(Clone)]
pub struct MockResponse {
    pub status: u16,
    pub body: String,
    pub delay: Option<std::time::Duration>,
}

#[derive(Clone)]
pub struct MockScraper {
    retry_config: RetryConfig,
    responses: Arc<Vec<MockResponse>>,
    current_response: Arc<std::sync::atomic::AtomicUsize>,
}

impl MockScraper {
    pub fn new(retry_config: RetryConfig, responses: Vec<MockResponse>) -> Self {
        Self {
            retry_config,
            responses: Arc::new(responses),
            current_response: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl Scraper for MockScraper {
    async fn fetch_single(&self, url: Url) -> ScraperResult<Response> {
        let index = self
            .current_response
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let response = &self.responses[index % self.responses.len()];

        if let Some(delay) = response.delay {
            sleep(delay).await;
        }

        Ok(Response {
            url,
            status: response.status,
            headers: HashMap::new(),
            body: response.body.clone(),
            timestamp: Utc::now(),
            retry_count: 0,
            retry_history: HashMap::new(),
        })
    }

    fn box_clone(&self) -> Box<dyn Scraper> {
        Box::new(self.clone())
    }

    fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }
}
