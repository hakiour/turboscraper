use crate::core::spider::{ParseResult, SpiderResponse};
use crate::stats::{ErrorType, StatsTracker};
use crate::storage::{StorageCategory, StorageItem};
use crate::{HttpRequest, HttpResponse, Scraper, ScraperError};
use chrono::Utc;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, error, info, trace, warn};
use parking_lot::RwLock;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::spawn;
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

        let retry_error = ScraperError::ParsingError("Content retry requested".to_string());

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
                spider_clone.process_response(&spider_response).await
            }));
        }
    }

    async fn check_and_process_retry<S: Spider + Send + Sync + 'static>(
        &self,
        request: HttpRequest,
        error: &ScraperError,
        spider: Arc<S>,
        futures: &mut FuturesUnordered<JoinHandle<ScraperResult<ParseResult>>>,
    ) {
        let config = spider.config();

        let error_item = StorageItem {
            url: request.url.clone(),
            timestamp: Utc::now(),
            data: json!({
                "error": format!("{:?}", error),
                "spider": spider.name(),
                "request": request,
                "raw_body": request.body,
            }),
            metadata: Some(json!({
                "error_type": match error {
                    ScraperError::ParsingError(_) => "parsing_error",
                    ScraperError::StorageError(_) => "storage_error",
                    _ => "other_error",
                },
                "depth": request.depth,
            })),
            id: format!("{}_errors", spider.name()),
        };

        if let Err(e) = spider
            .store_data(
                error_item,
                StorageCategory::Error,
                Box::new(request.clone()),
            )
            .await
        {
            error!("Failed to store error: {:?}", e);
        }

        if let Some((category, delay)) = config.retry_config.should_retry_parse(&request.url, error)
        {
            warn!(
                "Retrying request for URL: {} (category: {:?}, delay: {:?})",
                request.url, category, delay
            );
            sleep(delay).await;
            self.process_requests(vec![request], spider, futures, true)
                .await;
        } else {
            info!("No retry configuration matches error: {:?}", error);
        }
    }

    pub async fn run<S: Spider + Send + Sync + 'static>(&self, spider: S) -> ScraperResult<()> {
        let spider = Arc::new(spider);
        let mut futures = FuturesUnordered::new();

        info!("Starting spider: {}", spider.name());
        debug!("Max depth: {}", spider.config().max_depth);

        let initial_requests = spider.start_requests();
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
                        self.handle_same_content_retry(
                            *response,
                            Arc::clone(&spider),
                            &mut futures,
                        )
                        .await;
                    }
                    ParseResult::RetryWithNewContent(request) => {
                        self.check_and_process_retry(
                            *request,
                            &ScraperError::ParsingError(
                                "Retry with new content requested".to_string(),
                            ),
                            Arc::clone(&spider),
                            &mut futures,
                        )
                        .await;
                    }
                },
                Ok(Err((error, request))) => match error {
                    ScraperError::MaxRetriesReached { category, url, .. } => {
                        warn!(
                            "Maximum retries reached for URL: {} (category: {:?})",
                            url.to_string(),
                            category
                        );
                        spider.handle_max_retries(category, request).await?;
                    }
                    ScraperError::StorageError(msg) => {
                        warn!("Storage error processing request: {}", msg);
                        self.stats.record_error(ErrorType::Storage);
                        self.check_and_process_retry(
                            *request,
                            &ScraperError::StorageError(msg),
                            Arc::clone(&spider),
                            &mut futures,
                        )
                        .await;
                    }
                    ScraperError::ParsingError(msg) => {
                        warn!("Parsing error processing request: {}", msg);
                        self.check_and_process_retry(
                            *request,
                            &ScraperError::ParsingError(msg),
                            Arc::clone(&spider),
                            &mut futures,
                        )
                        .await;
                    }
                    _ => {
                        warn!("Unhandled error type: {:?}", error);
                        self.stats.record_error(ErrorType::Unhandled);
                    }
                },
                Err(e) => {
                    warn!("Task error: {}", e);
                    self.stats.record_error(ErrorType::Unhandled);
                }
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

            if !is_retry
                && !spider.config().allow_url_revisit
                && self.visited_urls.read().contains(&url_str)
            {
                debug!("Skipping URL {} - already visited", url_str);
                continue;
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

            self.process_request(request.clone(), Arc::clone(&spider), futures)
                .await;
        }
    }

    async fn process_request<S: Spider + Send + Sync + 'static>(
        &self,
        request: HttpRequest,
        spider: Arc<S>,
        futures: &mut FuturesUnordered<JoinHandle<ScraperResult<ParseResult>>>,
    ) {
        let spider_clone = Arc::clone(&spider);
        let scraper = self.scraper.box_clone();
        let config = spider.config().clone();
        let stats = Arc::clone(&self.stats);
        let start_time = Utc::now();

        futures.push(spawn(async move {
            let response = scraper.fetch(request.clone(), &config).await?;
            let spider_response = SpiderResponse {
                response: response.clone(),
                callback: request.callback.clone(),
            };
            let parse_result = spider_clone.process_response(&spider_response).await;
            let duration = Utc::now().signed_duration_since(start_time);

            // Record retry stats if any (moved outside match to avoid duplication)
            if response.retry_count > 0 {
                for (category, count) in response.retry_history.iter() {
                    for _ in 0..*count {
                        stats.record_retry(format!("{:?}", category));
                    }
                }
            }

            // Update stats based on parsing result and response
            match &parse_result {
                Ok(_) => {
                    stats.record_request(
                        response.status,
                        response.decoded_body.len(),
                        duration,
                        true, // Parsing succeeded
                    );
                }
                Err(_) => {
                    stats.record_error(ErrorType::Parsing);
                    stats.record_request(
                        response.status,
                        response.decoded_body.len(),
                        duration,
                        false, // Parsing failed
                    );
                }
            }

            parse_result
        }));
    }
}
