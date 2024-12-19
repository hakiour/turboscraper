use std::time::Duration;
use turboscraper::examples::example_spider::ExampleSpider;
use turboscraper::scraper::{http_scraper::HttpScraper, BackoffPolicy, RetryConfig};
use turboscraper::scraper::{ContentRetryCondition, RetryCondition};
use turboscraper::{errors::ScraperResult, Crawler};

#[actix_rt::main]
async fn main() -> ScraperResult<()> {
    env_logger::init();

    let retry_config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
        retry_conditions: vec![
            RetryCondition::StatusCode(429),
            RetryCondition::StatusCode(503),
            RetryCondition::Content(ContentRetryCondition {
                pattern: "bot detected".to_string(),
                is_regex: false,
            }),
            RetryCondition::Content(ContentRetryCondition {
                pattern: r"captcha|verify you're human".to_string(),
                is_regex: true,
            }),
        ],
        backoff_policy: BackoffPolicy::Exponential { factor: 2.0 },
    };

    let scraper = Box::new(HttpScraper::with_config(retry_config));
    let crawler = Crawler::new(scraper, 30);
    let spider = ExampleSpider::new()?;

    crawler.run(spider).await?;

    Ok(())
}
