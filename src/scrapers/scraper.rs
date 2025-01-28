use crate::core::spider::SpiderConfig;
use crate::http::request::HttpRequest;
use crate::{HttpResponse, ScraperError, ScraperResult, StatsTracker};
use async_trait::async_trait;
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
        let url = request.url.clone();

        loop {
            info!("Fetching URL: {} [{}]", url, request.method);
            let response = self.fetch_single(request.clone(), config).await?;
            debug!(
                "Received response: status={}, body_length={}",
                response.status,
                response.decoded_body.len()
            );

            if let Some((category, delay)) = config.retry_config.should_retry_request(
                &url,
                response.status,
                &response.decoded_body,
            ) {
                self.stats().record_retry(format!("{:?}", category));
                let state = config.retry_config.get_retry_state(&url);
                let attempt = state.counts.get(&category).unwrap();
                let max_retries = config
                    .retry_config
                    .categories
                    .get(&category)
                    .map(|c| c.max_retries)
                    .unwrap_or(0);

                if attempt >= &max_retries {
                    return Err((
                        ScraperError::MaxRetriesReached {
                            category: category.clone(),
                            retry_count: *attempt,
                            url: Box::new(url.clone()),
                        },
                        Box::new(request),
                    ));
                }

                warn!(
                    "Retry triggered for URL: {} (category={:?}, attempt={}/{}, delay={:?})",
                    url, category, attempt, max_retries, delay
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

            return Ok(HttpResponse {
                retry_count: state.total_retries,
                retry_history: state.counts,
                ..response
            });
        }
    }
}
