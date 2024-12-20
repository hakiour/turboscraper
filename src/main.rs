use std::time::Duration;
use turboscraper::examples::example_spiders::beginner::simple_spider::BookSpider;

use turboscraper::core::retry::{
    BackoffPolicy, CategoryConfig, ContentRetryCondition, RetryCategory, RetryCondition,
    RetryConfig,
};
use turboscraper::scrapers::http_scraper::HttpScraper;
use turboscraper::storage::{StorageType, create_storage};
use turboscraper::{Crawler, ScraperResult};
use turboscraper::core::spider::SpiderConfig;

#[actix_rt::main]
async fn main() -> ScraperResult<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("selectors", log::LevelFilter::Warn)
        .filter_module("html5ever", log::LevelFilter::Error)
        .init();

    let mut retry_config = RetryConfig::default();

    // Customize the rate limit category
    retry_config.categories.insert(
        RetryCategory::RateLimit,
        CategoryConfig {
            max_retries: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
            conditions: vec![
                RetryCondition::StatusCode(429),
                RetryCondition::Content(ContentRetryCondition {
                    pattern: "rate limit|too many requests".to_string(),
                    is_regex: true,
                }),
            ],
            backoff_policy: BackoffPolicy::Exponential { factor: 2.0 },
        },
    );

    let scraper = Box::new(HttpScraper::with_config(retry_config));
    let crawler = Crawler::new(scraper);

    // Choose storage type
    let storage = create_storage(StorageType::Disk {
        path: "data".to_string()
    }).await?;
    
    // Or use MongoDB
    /*
    let storage = create_storage(StorageType::Mongo {
        uri: "mongodb://localhost:27017".to_string(),
        database: "book_scraper".to_string(),
    }).await?;
    */

    let spider = BookSpider::new(storage)
        .await?
        .with_config(SpiderConfig {
            max_depth: 999999,
            max_concurrency: 30,
        });

    crawler.run(spider).await?;

    Ok(())
}
