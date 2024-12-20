use super::response::Response;
use super::retry::RetryConfig;
use crate::errors::ScraperResult;
use async_trait::async_trait;
use log::{debug, info, trace, warn};
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
            info!("Fetching URL: {}", url);
            let response = self.fetch_single(url.clone()).await?;
            debug!(
                "Received response: status={}, body_length={}",
                response.status,
                response.body.len()
            );

            if let Some((category, delay)) =
                self.retry_config()
                    .should_retry(response.status, &response.body, &mut retry_counts)
            {
                total_retries += 1;
                let attempt = retry_counts.get(&category).unwrap();

                warn!(
                    "Retry triggered for URL: {} (category={:?}, attempt={}/{}, delay={:?})",
                    url,
                    category,
                    attempt,
                    self.retry_config()
                        .categories
                        .get(&category)
                        .map(|c| c.max_retries)
                        .unwrap_or(0),
                    delay
                );

                if let Some(config) = self.retry_config().categories.get(&category) {
                    debug!(
                        "Retry config for {:?}: max_retries={}, current_attempt={}",
                        category, config.max_retries, attempt
                    );
                }

                sleep(delay).await;
                continue;
            }

            info!(
                "Request completed for URL: {} (total_retries={}, status={})",
                url, total_retries, response.status
            );
            debug!("Retry history: {:?}", retry_counts);

            trace!("Response content length: {} bytes", response.body.len());

            return Ok(Response {
                retry_count: total_retries,
                retry_history: retry_counts,
                ..response
            });
        }
    }
}
