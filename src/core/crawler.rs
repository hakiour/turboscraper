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
    concurrent_requests: usize,
    visited_urls: Arc<RwLock<HashSet<String>>>,
    stats: Arc<StatsTracker>,
}

impl Crawler {
    pub fn new(scraper: Box<dyn Scraper>, concurrent_requests: usize) -> Self {
        info!(
            "Initializing crawler with {} concurrent requests",
            concurrent_requests
        );
        let stats = Arc::new(StatsTracker::new());

        // Set the stats tracker in the scraper
        scraper.set_stats(Arc::clone(&stats));

        Self {
            scraper,
            concurrent_requests,
            visited_urls: Arc::new(RwLock::new(HashSet::new())),
            stats,
        }
    }

    pub async fn run<S: Spider + Send + Sync + 'static>(&self, spider: S) -> ScraperResult<()> {
        let spider = Arc::new(spider);
        let mut futures = FuturesUnordered::new();

        info!("Starting spider: {}", spider.name());
        debug!("Max depth: {}", spider.max_depth());

        // Initialize with start URLs at depth 0
        for url in spider.start_urls() {
            let spider_clone = Arc::clone(&spider);
            let scraper = self.scraper.box_clone();
            let visited = Arc::clone(&self.visited_urls);

            info!("Adding start URL: {}", url);
            visited.write().insert(url.to_string());

            futures.push(spawn(async move {
                let response = scraper.fetch(url.clone()).await?;
                trace!(
                    "Response content length for {}: {} bytes",
                    url,
                    response.body.len()
                );
                spider_clone.parser().parse(response, url, 0).await
            }));
        }

        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(new_requests)) => {
                    debug!("Found {} new URLs to process", new_requests.len());

                    for request in new_requests {
                        if request.depth >= spider.max_depth() {
                            debug!("Skipping URL {} - max depth reached", request.url);
                            continue;
                        }

                        let url_str = request.url.to_string();
                        if self.visited_urls.read().contains(&url_str) {
                            debug!("Skipping URL {} - already visited", url_str);
                            continue;
                        }

                        info!("Processing new URL: {} at depth {}", url_str, request.depth);
                        if let Some(meta) = &request.meta {
                            trace!("Request metadata: {:?}", meta);
                        }

                        self.visited_urls.write().insert(url_str);

                        if futures.len() >= self.concurrent_requests {
                            debug!("Reached concurrent request limit, waiting for slot");
                            futures.next().await;
                        }

                        let spider_clone = Arc::clone(&spider);
                        let scraper = self.scraper.box_clone();
                        let depth = request.depth;

                        futures.push(spawn(async move {
                            let response = scraper.fetch(request.url.clone()).await?;
                            trace!("Response content length: {} bytes", response.body.len());
                            spider_clone.parser().parse(response, request.url, depth).await
                        }));
                    }
                }
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
