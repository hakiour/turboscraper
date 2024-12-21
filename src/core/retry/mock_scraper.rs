#[cfg(test)]
use crate::core::retry::RetryConfig;
#[cfg(test)]
use crate::{Response, Scraper, ScraperResult, StatsTracker};
#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use chrono::Utc;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use std::sync::RwLock;
#[cfg(test)]
use tokio::time::sleep;
#[cfg(test)]
use url::Url;

#[cfg(test)]
#[derive(Clone)]
pub struct MockResponse {
    pub status: u16,
    pub body: String,
    pub delay: Option<std::time::Duration>,
}

#[cfg(test)]
#[derive(Clone)]
pub struct MockScraper {
    retry_config: RetryConfig,
    responses: Arc<Vec<MockResponse>>,
    current_response: Arc<std::sync::atomic::AtomicUsize>,
    stats: Arc<RwLock<Arc<StatsTracker>>>,
}

#[cfg(test)]
impl MockScraper {
    pub fn new(retry_config: RetryConfig, responses: Vec<MockResponse>) -> Self {
        Self {
            retry_config,
            responses: Arc::new(responses),
            current_response: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            stats: Arc::new(RwLock::new(Arc::new(StatsTracker::new()))),
        }
    }
}

#[cfg(test)]
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
            meta: None,
        })
    }

    fn box_clone(&self) -> Box<dyn Scraper> {
        Box::new(self.clone())
    }

    fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }
    fn stats(&self) -> &StatsTracker {
        static STATS: std::sync::OnceLock<StatsTracker> = std::sync::OnceLock::new();
        STATS.get_or_init(|| (*self.stats.read().unwrap()).as_ref().clone())
    }

    fn set_stats(&self, stats: Arc<StatsTracker>) {
        *self.stats.write().unwrap() = stats;
    }
}
