use crate::core::spider::SpiderConfig;
use crate::http::request::HttpRequest;
use crate::{HttpResponse, ScraperResult, StatsTracker};
use async_trait::async_trait;
use chrono::Utc;
use log::{debug, info, warn};
use std::sync::Arc;
use tokio::time::sleep;

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn fetch_single(
        &self,
        request: HttpRequest,
        config: &SpiderConfig,
    ) -> ScraperResult<HttpResponse>;
    fn box_clone(&self) -> Box<dyn Scraper>;
    fn stats(&self) -> &StatsTracker;
    fn set_stats(&mut self, stats: Arc<StatsTracker>);

    async fn fetch(
        &self,
        request: HttpRequest,
        config: &SpiderConfig,
    ) -> ScraperResult<HttpResponse> {
        let start_time = Utc::now();
        let url = request.url.clone();

        loop {
            info!("Fetching URL: {} [{}]", url, request.method);
            let response = self.fetch_single(request.clone(), config).await?;
            debug!(
                "Received response: status={}, body_length={}",
                response.status,
                response.body.len()
            );

            if let Some((category, delay)) =
                config
                    .retry_config
                    .should_retry(&url, response.status, &response.body)
            {
                self.stats().record_retry(format!("{:?}", category));
                let state = config.retry_config.get_retry_state(&url);
                let attempt = state.counts.get(&category).unwrap();

                warn!(
                    "Retry triggered for URL: {} (category={:?}, attempt={}/{}, delay={:?})",
                    url,
                    category,
                    attempt,
                    config
                        .retry_config
                        .categories
                        .get(&category)
                        .map(|c| c.max_retries)
                        .unwrap_or(0),
                    delay
                );

                sleep(delay).await;
                continue;
            }

            let state = config.retry_config.get_retry_state(&url);

            info!(
                "Request completed for URL: {} (total_retries={}, status={})",
                url, state.total_retries, response.status
            );
            debug!("Retry history for {}: {:?}", url, state.counts);

            let duration = Utc::now().signed_duration_since(start_time);
            self.stats()
                .record_request(response.status, response.body.len(), duration);

            return Ok(HttpResponse {
                retry_count: state.total_retries,
                retry_history: state.counts,
                ..response
            });
        }
    }
}
