use std::time::Duration;
use turboscraper::examples::example_spiders::beginner::simple_spider::BookSpider;

use turboscraper::core::retry::{
    BackoffPolicy, CategoryConfig, ContentRetryCondition, RetryCategory, RetryCondition,
    RetryConfig,
};
use turboscraper::core::spider::SpiderConfig;
use turboscraper::scrapers::http_scraper::HttpScraper;
use turboscraper::storage::{create_storage, StorageType};
use turboscraper::{Crawler, ScraperResult, Spider};

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

    let spider_config = SpiderConfig::default()
        .with_retry(retry_config)
        .with_depth(999999)
        .with_concurrency(30)
        .with_headers(vec![("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")]);

    let storage = create_storage(StorageType::Disk {
        path: "data".to_string(),
    })
    .await?;

    // Or use MongoDB
    /*
    let storage = create_storage(StorageType::Mongo {
        uri: "mongodb://localhost:27017".to_string(),
        database: "book_scraper".to_string(),
    }).await?;
    */

    let spider = BookSpider::new(storage).unwrap().with_config(spider_config);

    let scraper = Box::new(HttpScraper::new());
    let crawler = Crawler::new(scraper);
    crawler.run(spider).await?;

    Ok(())
}
