use crate::{Response, ScraperResult, StatsTracker};
use crate::core::retry::RetryConfig;
use async_trait::async_trait;
use chrono::Utc;
use log::{debug, info, warn};
use std::sync::Arc;
use tokio::time::sleep;
use url::Url;

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn fetch_single(&self, url: Url) -> ScraperResult<Response>;
    fn box_clone(&self) -> Box<dyn Scraper>;
    fn retry_config(&self) -> &RetryConfig;
    fn stats(&self) -> &StatsTracker;
    fn set_stats(&self, stats: Arc<StatsTracker>);

    async fn fetch(&self, url: Url) -> ScraperResult<Response> {
        let start_time = Utc::now();

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
                    .should_retry(&url, response.status, &response.body)
            {
                self.stats().record_retry(format!("{:?}", category));
                let state = self.retry_config().get_retry_state(&url);
                let attempt = state.counts.get(&category).unwrap();

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

                sleep(delay).await;
                continue;
            }

            let state = self.retry_config().get_retry_state(&url);

            info!(
                "Request completed for URL: {} (total_retries={}, status={})",
                url, state.total_retries, response.status
            );
            debug!("Retry history for {}: {:?}", url, state.counts);

            let duration = Utc::now().signed_duration_since(start_time);
            self.stats()
                .record_request(response.status, response.body.len(), duration);

            return Ok(Response {
                retry_count: state.total_retries,
                retry_history: state.counts,
                ..response
            });
        }
    }
}
