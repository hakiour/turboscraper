use super::response::Response;
use super::retry::RetryConfig;
use crate::errors::ScraperResult;
use async_trait::async_trait;
use log;
use tokio::time::sleep;
use url::Url;

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn fetch_single(&self, url: Url) -> ScraperResult<Response>;
    fn box_clone(&self) -> Box<dyn Scraper>;
    fn retry_config(&self) -> &RetryConfig;

    async fn fetch(&self, url: Url) -> ScraperResult<Response> {
        let mut retry_count = 0;

        loop {
            let response = self.fetch_single(url.clone()).await?;

            if !self
                .retry_config()
                .should_retry(response.status, &response.body, retry_count)
            {
                return Ok(Response {
                    retry_count,
                    ..response
                });
            }

            retry_count += 1;
            let delay = self.retry_config().calculate_delay(retry_count);

            log::warn!(
                "Retrying request to {} (attempt {}/{}) after {:?}",
                url,
                retry_count,
                self.retry_config().max_retries,
                delay
            );

            sleep(delay).await;
        }
    }
}
