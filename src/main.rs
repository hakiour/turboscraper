use std::time::Duration;
use turboscraper::examples::example_spiders::beginner::simple_spider::BookSpider;

use turboscraper::core::retry::{
    BackoffPolicy, CategoryConfig, ContentRetryCondition, RetryCategory, RetryCondition,
    RetryConfig,
};
use turboscraper::scrapers::http_scraper::HttpScraper;
use turboscraper::{Crawler, ScraperResult};

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
    let crawler: Crawler = Crawler::new(scraper, 10);
    let spider = BookSpider::new()?.with_max_depth(999999);

    crawler.run(spider).await?;

    Ok(())
}
