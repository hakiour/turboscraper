use crate::core::spider::{ParseResult, SpiderResponse};
use crate::stats::StatsTracker;
use crate::Scraper;
use actix_rt::spawn;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, info, trace, warn};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;

use super::{ScraperResult, Spider};

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

    pub async fn run<S: Spider + Send + Sync + 'static>(&self, spider: S) -> ScraperResult<()> {
        let spider = Arc::new(spider);
        let mut futures = FuturesUnordered::new();

        info!("Starting spider: {}", spider.name());
        debug!("Max depth: {}", spider.config().max_depth);

        // Use spider's method to get initial requests
        for request in spider.get_initial_requests() {
            let spider_clone = Arc::clone(&spider);
            let scraper = self.scraper.box_clone();
            let visited = Arc::clone(&self.visited_urls);
            let config = spider.config().clone();

            info!("Adding start URL: {}", request.url);
            visited.write().insert(request.url.to_string());

            futures.push(spawn(async move {
                let response = scraper.fetch(request.clone(), &config).await?;
                let spider_response = SpiderResponse {
                    response,
                    callback: request.callback.clone(),
                };
                spider_clone.parse(spider_response, request.url, 0).await
            }));
        }

        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(parse_result)) => match parse_result {
                    ParseResult::Continue(new_requests) => {
                        debug!("Found {} new URLs to process", new_requests.len());
                        for request in new_requests {
                            if request.depth >= spider.config().max_depth {
                                debug!("Skipping URL {} - max depth reached", request.url);
                                continue;
                            }

                            let url_str = request.url.to_string();
                            if !spider.config().allow_url_revisit {
                                if self.visited_urls.read().contains(&url_str) {
                                    debug!("Skipping URL {} - already visited", url_str);
                                    continue;
                                }
                            }

                            info!("Processing new URL: {} at depth {}", url_str, request.depth);
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

                            let spider_clone = Arc::clone(&spider);
                            let scraper = self.scraper.box_clone();
                            let depth = request.depth;
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
                    ParseResult::Skip => {
                        debug!("Skipping current URL");
                        continue;
                    }
                    ParseResult::Stop => {
                        info!("Spider requested stop");
                        break;
                    }
                },
                Ok(Err(e)) => warn!("Error processing request: {}", e),
                Err(e) => warn!("Task error: {}", e),
            }
        }

        info!(
            "Spider {} completed. Total URLs processed: {}",
            spider.name(),
            self.visited_urls.read().len()
        );
        self.stats.finish();
        self.stats.print_summary();
        Ok(())
    }
}
