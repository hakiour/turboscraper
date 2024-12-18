use super::*;
use crate::scraper::mock_scraper::{MockResponse, MockScraper};
use std::time::Duration;
use url::Url;

#[tokio::test]
async fn test_status_code_retry() {
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

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        retry_conditions: vec![RetryCondition::StatusCode(429)],
        backoff_policy: BackoffPolicy::Constant,
    };

    let scraper = MockScraper::new(retry_config, responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper.fetch(url).await.unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "Success");
    assert_eq!(response.retry_count, 1);
}

#[tokio::test]
async fn test_content_based_retry() {
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

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        retry_conditions: vec![RetryCondition::Content(ContentRetryCondition {
            pattern: "bot detected".to_string(),
            is_regex: false,
        })],
        backoff_policy: BackoffPolicy::Constant,
    };

    let scraper = MockScraper::new(retry_config, responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper.fetch(url).await.unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "Welcome user");
    assert_eq!(response.retry_count, 1);
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

    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        retry_conditions: vec![RetryCondition::StatusCode(429)],
        backoff_policy: BackoffPolicy::Exponential { factor: 2.0 },
    };

    let start = std::time::Instant::now();
    let scraper = MockScraper::new(retry_config, responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper.fetch(url).await.unwrap();

    let elapsed = start.elapsed();
    assert_eq!(response.status, 200);
    assert_eq!(response.retry_count, 2);
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

    let retry_config = RetryConfig {
        max_retries: 2,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        retry_conditions: vec![RetryCondition::StatusCode(429)],
        backoff_policy: BackoffPolicy::Constant,
    };

    let scraper = MockScraper::new(retry_config, responses);
    let url = Url::parse("https://example.com").unwrap();
    let response = scraper.fetch(url).await.unwrap();

    assert_eq!(response.status, 429);
    assert_eq!(response.retry_count, 2);
} 