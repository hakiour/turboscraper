use crate::errors::ScraperResult;
use crate::scraper::Scraper;
use crate::spider::Spider;
use actix_rt::spawn;
use futures::stream::{FuturesUnordered, StreamExt};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;

pub struct Crawler {
    scraper: Box<dyn Scraper>,
    concurrent_requests: usize,
    visited_urls: Arc<RwLock<HashSet<String>>>,
}

impl Crawler {
    pub fn new(scraper: Box<dyn Scraper>, concurrent_requests: usize) -> Self {
        Self {
            scraper,
            concurrent_requests,
            visited_urls: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn run<S: Spider + Send + Sync + 'static>(&self, spider: S) -> ScraperResult<()> {
        let spider = Arc::new(spider);
        let mut futures = FuturesUnordered::new();

        // Initialize with start URLs at depth 0
        for url in spider.start_urls() {
            let spider_clone = Arc::clone(&spider);
            let scraper = self.scraper.box_clone();
            let visited = Arc::clone(&self.visited_urls);

            // Mark URL as visited
            visited.write().insert(url.to_string());

            futures.push(spawn(async move {
                let response = scraper.fetch(url.clone()).await?;
                spider_clone.parse(response, url, 0).await
            }));
        }

        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(new_requests)) => {
                    // Handle new requests discovered during parsing
                    for request in new_requests {
                        // Skip if max depth reached or URL already visited
                        if request.depth >= spider.max_depth() {
                            continue;
                        }

                        let url_str = request.url.to_string();
                        if self.visited_urls.read().contains(&url_str) {
                            continue;
                        }

                        // Mark as visited
                        self.visited_urls.write().insert(url_str);

                        if futures.len() >= self.concurrent_requests {
                            futures.next().await;
                        }

                        let spider_clone = Arc::clone(&spider);
                        let scraper = self.scraper.box_clone();
                        let depth = request.depth;

                        futures.push(spawn(async move {
                            let response = scraper.fetch(request.url.clone()).await?;
                            spider_clone.parse(response, request.url, depth).await
                        }));
                    }
                }
                Ok(Err(e)) => log::error!("Error processing request: {}", e),
                Err(e) => log::error!("Task error: {}", e),
            }
        }

        Ok(())
    }
}
