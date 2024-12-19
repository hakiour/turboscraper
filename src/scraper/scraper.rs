use super::response::Response;
use super::retry::RetryConfig;
use crate::errors::ScraperResult;
use async_trait::async_trait;
use log;
use std::collections::HashMap;
use tokio::time::sleep;
use url::Url;

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn fetch_single(&self, url: Url) -> ScraperResult<Response>;
    fn box_clone(&self) -> Box<dyn Scraper>;
    fn retry_config(&self) -> &RetryConfig;

    async fn fetch(&self, url: Url) -> ScraperResult<Response> {
        let mut retry_counts = HashMap::new();
        let mut total_retries = 0;

        loop {
            let response = self.fetch_single(url.clone()).await?;

            if let Some((category, delay)) =
                self.retry_config()
                    .should_retry(response.status, &response.body, &mut retry_counts)
            {
                total_retries += 1;

                log::warn!(
                    "Retrying request to {} (category: {:?}, attempt {}) after {:?}",
                    url,
                    category,
                    retry_counts.get(&category).unwrap(),
                    delay
                );

                sleep(delay).await;
                continue;
            }

            return Ok(Response {
                retry_count: total_retries,
                retry_history: retry_counts,
                ..response
            });
        }
    }
}
