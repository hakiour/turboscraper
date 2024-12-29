use crate::core::spider::{ParseResult, SpiderResponse};
use crate::stats::StatsTracker;
use crate::{HttpRequest, HttpResponse, Scraper, ScraperError};
use actix_rt::spawn;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, info, trace, warn};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::{ScraperResult, Spider};

pub struct Crawler {
    scraper: Box<dyn Scraper>,
    visited_urls: Arc<RwLock<HashSet<String>>>,
    stats: Arc<StatsTracker>,
}

impl Crawler {
    pub fn new(scraper: Box<dyn Scraper>) -> Self {
        info!("Initializing crawler");
        let stats = Arc::new(StatsTracker::new());
        let mut scraper = scraper;
        scraper.set_stats(Arc::clone(&stats));

        Self {
            scraper,
            visited_urls: Arc::new(RwLock::new(HashSet::new())),
            stats,
        }
    }

    async fn handle_same_content_retry<S: Spider + Send + Sync + 'static>(
        &self,
        response: HttpResponse,
        spider: Arc<S>,
        futures: &mut FuturesUnordered<JoinHandle<ScraperResult<ParseResult>>>,
    ) {
        let spider_clone = Arc::clone(&spider);
        let config = spider.config().clone();

        let retry_error = ScraperError::ExtractionError("Content retry requested".to_string());

        if let Some((category, delay)) = config
            .retry_config
            .should_retry_parse(&response.url, &retry_error)
        {
            warn!(
                "Retrying parse with same content for URL: {} (category: {:?})",
                response.url, category
            );
            sleep(delay).await;

            let spider_response = SpiderResponse {
                response: response.clone(),
                callback: response.from_request.callback.clone(),
            };

            futures.push(spawn(async move {
                spider_clone
                    .parse(spider_response, response.url, response.from_request.depth)
                    .await
            }));
        }
    }

    pub async fn run<S: Spider + Send + Sync + 'static>(&self, spider: S) -> ScraperResult<()> {
        let spider = Arc::new(spider);
        let mut futures = FuturesUnordered::new();

        info!("Starting spider: {}", spider.name());
        debug!("Max depth: {}", spider.config().max_depth);

        let initial_requests = spider.get_initial_requests();
        if !initial_requests.is_empty() {
            self.process_requests(initial_requests, Arc::clone(&spider), &mut futures, false)
                .await;
        }

        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(parse_result)) => match parse_result {
                    ParseResult::Continue(new_requests) => {
                        self.process_requests(
                            new_requests,
                            Arc::clone(&spider),
                            &mut futures,
                            false,
                        )
                        .await;
                    }
                    ParseResult::Skip => {
                        debug!("Skipping current URL");
                        continue;
                    }
                    ParseResult::Stop => {
                        info!("Spider requested stop");
                        break;
                    }
                    ParseResult::RetryWithSameContent(response) => {
                        self.handle_same_content_retry(response, Arc::clone(&spider), &mut futures)
                            .await;
                    }
                    ParseResult::RetryWithNewContent(request) => {
                        // Reuse process_requests for new content retries
                        self.process_requests(
                            vec![request],
                            Arc::clone(&spider),
                            &mut futures,
                            true,
                        )
                        .await;
                    }
                },
                Ok(Err(e)) => {
                    warn!("Error processing request: {}", e.0);
                    match e.0 {
                        ScraperError::StorageError(_) => {
                            self.stats.increment_storage_errors();
                            self.process_requests(
                                vec![e.1],
                                Arc::clone(&spider),
                                &mut futures,
                                true,
                            )
                            .await;
                        }
                        _ => {
                            // Handle other types of errors
                        }
                    }
                }
                Err(e) => warn!("Task error: {}", e),
            }
        }

        info!(
            "Spider {} completed. Total URLs processed: {}",
            spider.name(),
            self.visited_urls.read().len()
        );
        self.stats.print_summary();
        Ok(())
    }

    async fn process_requests<S: Spider + Send + Sync + 'static>(
        &self,
        requests: Vec<HttpRequest>,
        spider: Arc<S>,
        futures: &mut FuturesUnordered<JoinHandle<ScraperResult<ParseResult>>>,
        is_retry: bool,
    ) {
        for request in requests {
            if request.depth >= spider.config().max_depth {
                debug!("Skipping URL {} - max depth reached", request.url);
                continue;
            }

            let url_str = request.url.to_string();

            if !is_retry && !spider.config().allow_url_revisit {
                if self.visited_urls.read().contains(&url_str) {
                    debug!("Skipping URL {} - already visited", url_str);
                    continue;
                }
            }

            info!("Processing URL: {} at depth {}", url_str, request.depth);
            if let Some(meta) = &request.meta {
                trace!("Request metadata: {:?}", meta);
            }

            self.visited_urls.write().insert(url_str);

            if futures.len() >= spider.config().max_concurrency {
                debug!(
                    "Reached concurrent request limit {}, waiting for slot",
                    spider.config().max_concurrency
                );
                futures.next().await;
            }

            self.process_request(request.clone(), Arc::clone(&spider), futures, request.depth)
                .await;
        }
    }

    async fn process_request<S: Spider + Send + Sync + 'static>(
        &self,
        request: HttpRequest,
        spider: Arc<S>,
        futures: &mut FuturesUnordered<JoinHandle<ScraperResult<ParseResult>>>,
        depth: usize,
    ) {
        let spider_clone = Arc::clone(&spider);
        let scraper = self.scraper.box_clone();
        let config = spider.config().clone();

        futures.push(spawn(async move {
            let response = scraper.fetch(request.clone(), &config).await?;
            let spider_response = SpiderResponse {
                response,
                callback: request.callback.clone(),
            };
            spider_clone
                .parse(spider_response, request.url, depth)
                .await
        }));
    }
}
