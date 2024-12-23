use crate::core::retry::{
    BackoffPolicy, CategoryConfig, ContentRetryCondition, RetryCategory, RetryCondition,
    RetryConfig,
};
use crate::core::spider::SpiderConfig;
use crate::core::SpiderCallback;
use crate::http::HttpRequest;
use crate::{
    core::retry::mock_scraper::{MockResponse, MockScraper},
    Scraper,
};
use std::time::Duration;
use url::Url;

#[tokio::test]
async fn test_rate_limit_retry() {
    let responses = vec![
        MockResponse {
            status: 429,
            body: "Rate limited".to_string(),
            delay: None,
        },
        MockResponse {
            status: 200,
            body: "Success".to_string(),
            delay: None,
        },
    ];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::RateLimit,
        CategoryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            conditions: vec![RetryCondition::StatusCode(429)],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let scraper = MockScraper::new(responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper
        .fetch(
            HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
            &SpiderConfig {
                retry_config,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "Success");
    assert_eq!(response.retry_count, 1);
    assert_eq!(
        response.retry_history.get(&RetryCategory::RateLimit),
        Some(&1)
    );
}

#[tokio::test]
async fn test_bot_detection_retry() {
    let responses = vec![
        MockResponse {
            status: 200,
            body: "Bot detected, please try again".to_string(),
            delay: None,
        },
        MockResponse {
            status: 200,
            body: "Welcome user".to_string(),
            delay: None,
        },
    ];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::BotDetection,
        CategoryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            conditions: vec![RetryCondition::Content(ContentRetryCondition {
                pattern: "bot detected".to_string(),
                is_regex: false,
            })],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let scraper = MockScraper::new(responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper
        .fetch(
            HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
            &SpiderConfig {
                retry_config,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "Welcome user");
    assert_eq!(response.retry_count, 1);
    assert_eq!(
        response.retry_history.get(&RetryCategory::BotDetection),
        Some(&1)
    );
}

#[tokio::test]
async fn test_exponential_backoff() {
    let responses = vec![
        MockResponse {
            status: 429,
            body: "Rate limited".to_string(),
            delay: None,
        },
        MockResponse {
            status: 429,
            body: "Rate limited".to_string(),
            delay: None,
        },
        MockResponse {
            status: 200,
            body: "Success".to_string(),
            delay: None,
        },
    ];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::RateLimit,
        CategoryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            conditions: vec![RetryCondition::StatusCode(429)],
            backoff_policy: BackoffPolicy::Exponential { factor: 2.0 },
        },
    );

    let start = std::time::Instant::now();
    let scraper = MockScraper::new(responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper
        .fetch(
            HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
            &SpiderConfig {
                retry_config,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let elapsed = start.elapsed();
    assert_eq!(response.status, 200);
    assert_eq!(response.retry_count, 2);
    assert_eq!(
        response.retry_history.get(&RetryCategory::RateLimit),
        Some(&2)
    );
    // Should wait ~300ms total (100ms + 200ms)
    assert!(elapsed >= Duration::from_millis(300));
}

#[tokio::test]
async fn test_max_retries_exceeded() {
    let responses = vec![MockResponse {
        status: 429,
        body: "Rate limited".to_string(),
        delay: None,
    }];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::RateLimit,
        CategoryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            conditions: vec![RetryCondition::StatusCode(429)],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let scraper = MockScraper::new(responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper
        .fetch(
            HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
            &SpiderConfig {
                retry_config,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(response.status, 429);
    assert_eq!(response.retry_count, 2);
    assert_eq!(
        response.retry_history.get(&RetryCategory::RateLimit),
        Some(&2)
    );
}

#[tokio::test]
async fn test_multiple_retry_categories() {
    let responses = vec![
        MockResponse {
            status: 429, // First rate limit
            body: "Rate limited".to_string(),
            delay: None,
        },
        MockResponse {
            status: 200, // Then bot detection
            body: "Bot detected, please verify".to_string(),
            delay: None,
        },
        MockResponse {
            status: 200, // Finally success
            body: "Success".to_string(),
            delay: None,
        },
    ];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::RateLimit,
        CategoryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            conditions: vec![RetryCondition::StatusCode(429)],
            backoff_policy: BackoffPolicy::Constant,
        },
    );
    retry_config.categories.insert(
        RetryCategory::BotDetection,
        CategoryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            conditions: vec![RetryCondition::Content(ContentRetryCondition {
                pattern: "Bot detected".to_string(),
                is_regex: false,
            })],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let scraper = MockScraper::new(responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper
        .fetch(
            HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
            &SpiderConfig {
                retry_config,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "Success");
    assert_eq!(response.retry_count, 2);
    assert_eq!(
        response.retry_history.get(&RetryCategory::RateLimit),
        Some(&1)
    );
    assert_eq!(
        response.retry_history.get(&RetryCategory::BotDetection),
        Some(&1)
    );
}

#[tokio::test]
async fn test_regex_content_retry() {
    let responses = vec![
        MockResponse {
            status: 200,
            body: "Your IP (1.2.3.4) has been blocked".to_string(),
            delay: None,
        },
        MockResponse {
            status: 200,
            body: "Success".to_string(),
            delay: None,
        },
    ];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::Blacklisted,
        CategoryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            conditions: vec![RetryCondition::Content(ContentRetryCondition {
                pattern: r"IP.*blocked".to_string(),
                is_regex: true,
            })],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let scraper = MockScraper::new(responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper
        .fetch(
            HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
            &SpiderConfig {
                retry_config,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "Success");
    assert_eq!(response.retry_count, 1);
    assert_eq!(
        response.retry_history.get(&RetryCategory::Blacklisted),
        Some(&1)
    );
}

#[tokio::test]
async fn test_custom_category() {
    let responses = vec![
        MockResponse {
            status: 200,
            body: "Checking your browser - Cloudflare".to_string(),
            delay: None,
        },
        MockResponse {
            status: 200,
            body: "Success".to_string(),
            delay: None,
        },
    ];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::Custom("CloudflareCheck".to_string()),
        CategoryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            conditions: vec![RetryCondition::Content(ContentRetryCondition {
                pattern: "Checking your browser.*Cloudflare".to_string(),
                is_regex: true,
            })],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let scraper = MockScraper::new(responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper
        .fetch(
            HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
            &SpiderConfig {
                retry_config,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "Success");
    assert_eq!(response.retry_count, 1);
    assert_eq!(
        response
            .retry_history
            .get(&RetryCategory::Custom("CloudflareCheck".to_string())),
        Some(&1)
    );
}

#[tokio::test]
async fn test_no_matching_retry_condition() {
    let responses = vec![MockResponse {
        status: 404, // Not configured for retry
        body: "Not Found".to_string(),
        delay: None,
    }];

    let retry_config = RetryConfig::default();
    let scraper = MockScraper::new(responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper
        .fetch(
            HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
            &SpiderConfig {
                retry_config,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(response.status, 404);
    assert_eq!(response.retry_count, 0);
    assert!(response.retry_history.is_empty());
}
