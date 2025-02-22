use crate::core::retry::mock_scraper::{MockResponse, MockScraper};
use crate::core::retry::{
    BackoffPolicy, CategoryConfig, ContentRetryCondition, ParseRetryCondition, ParseRetryType,
    RetryCategory, RetryCondition, RetryConfig,
};
use crate::core::spider::{ParseResult, ParsedData, SpiderCallback, SpiderConfig, SpiderResponse};
use crate::http::request::HttpRequest;
use crate::storage::base::StorageError;
use crate::storage::StorageManager;
use crate::{Crawler, ScraperError, ScraperResult, Spider};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

struct TestSpider {
    config: SpiderConfig,
    retry_count: Arc<RwLock<usize>>,
    retry_behavior: RetryBehavior,
}

enum RetryBehavior {
    NoRetry,
    RetryWithSame {
        max_attempts: usize,
        error: Option<ScraperError>,
    },
    RetryWithNew {
        max_attempts: usize,
        error: Option<ScraperError>,
    },
}

impl TestSpider {
    fn new(retry_count: Arc<RwLock<usize>>, behavior: RetryBehavior) -> Self {
        Self {
            config: SpiderConfig::default(),
            retry_count,
            retry_behavior: behavior,
        }
    }

    fn new_with_same_content(retry_count: Arc<RwLock<usize>>, max_attempts: usize) -> Self {
        Self::new(
            retry_count,
            RetryBehavior::RetryWithSame {
                max_attempts,
                error: None,
            },
        )
    }

    fn new_with_new_content(retry_count: Arc<RwLock<usize>>, max_attempts: usize) -> Self {
        Self::new(
            retry_count,
            RetryBehavior::RetryWithNew {
                max_attempts,
                error: None,
            },
        )
    }

    fn new_with_storage_error(retry_count: Arc<RwLock<usize>>, max_attempts: usize) -> Self {
        Self::new(
            retry_count,
            RetryBehavior::RetryWithSame {
                max_attempts,
                error: Some(ScraperError::StorageError(StorageError::OperationError(
                    "test storage error".to_string(),
                ))),
            },
        )
    }
}

#[async_trait]
impl Spider for TestSpider {
    fn name(&self) -> String {
        "test_spider".to_string()
    }

    fn storage_manager(&self) -> &StorageManager {
        unimplemented!("Storage manager not needed for test spider")
    }

    fn start_requests(&self) -> Vec<HttpRequest> {
        vec![HttpRequest::new(
            Url::parse("http://example.com").unwrap(),
            SpiderCallback::Bootstrap,
            0,
        )]
    }

    fn config(&self) -> &SpiderConfig {
        &self.config
    }

    fn set_config(&mut self, config: SpiderConfig) {
        self.config = config;
    }

    fn parse(&self, response: &SpiderResponse) -> ScraperResult<(ParseResult, ParsedData)> {
        let mut count = self.retry_count.write();
        *count += 1;

        let parsed_data = ParsedData::Empty;
        let parse_result = match &self.retry_behavior {
            RetryBehavior::NoRetry => ParseResult::Skip,
            RetryBehavior::RetryWithSame {
                max_attempts,
                error,
            } => {
                if *count < *max_attempts {
                    if error.is_some() {
                        return ScraperResult::Err((
                            ScraperError::StorageError(StorageError::OperationError(
                                "test storage error".to_string(),
                            )),
                            response.response.from_request.clone(),
                        ));
                    }
                    ParseResult::RetryWithSameContent(Box::new(response.response.clone()))
                } else {
                    ParseResult::Skip
                }
            }
            RetryBehavior::RetryWithNew {
                max_attempts,
                error,
            } => {
                if *count < *max_attempts {
                    if error.is_some() {
                        return ScraperResult::Err((
                            ScraperError::StorageError(StorageError::OperationError(
                                "test storage error".to_string(),
                            )),
                            response.response.from_request.clone(),
                        ));
                    }
                    let request = HttpRequest::new(
                        response.response.from_request.url.clone(),
                        SpiderCallback::ParseItem,
                        response.response.from_request.depth,
                    );
                    ParseResult::RetryWithNewContent(Box::new(request))
                } else {
                    ParseResult::Skip
                }
            }
        };

        Ok((parse_result, parsed_data))
    }

    async fn persist_extracted_data(
        &self,
        _data: ParsedData,
        _response: &SpiderResponse,
    ) -> ScraperResult<()> {
        Ok(())
    }

    async fn handle_max_retries(
        &self,
        _category: RetryCategory,
        _request: Box<HttpRequest>,
    ) -> ScraperResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_crawler_retry_with_same_content() {
    let retry_count = Arc::new(RwLock::new(0));
    let max_attempts = 3;
    let spider = TestSpider::new_with_same_content(Arc::clone(&retry_count), max_attempts);

    let mock_responses = vec![MockResponse {
        status: 200,
        body: "test content".to_string(),
        delay: None,
    }];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::ParseError,
        CategoryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1),
            conditions: vec![RetryCondition::Parse(ParseRetryCondition::Content(
                ContentRetryCondition {
                    pattern: "retry".to_string(),
                    is_regex: false,
                },
                ParseRetryType::SameContent,
            ))],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let config = SpiderConfig::default().with_retry(retry_config);
    let spider = spider.with_config(config);

    let scraper = Box::new(MockScraper::new(mock_responses));
    let crawler = Crawler::new(scraper);

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            crawler.run(spider).await.unwrap();

            assert_eq!(
                *retry_count.read(),
                max_attempts,
                "Expected {} attempts (initial + {} retries)",
                max_attempts,
                max_attempts - 1
            );
        })
        .await;
}

#[tokio::test]
async fn test_crawler_retry_with_new_content() {
    let retry_count = Arc::new(RwLock::new(0));
    let spider = TestSpider::new_with_new_content(Arc::clone(&retry_count), 3);

    let mock_responses = vec![MockResponse {
        status: 200,
        body: "first response".to_string(),
        delay: None,
    }];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::ParseError,
        CategoryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1),
            conditions: vec![RetryCondition::Parse(ParseRetryCondition::Content(
                ContentRetryCondition {
                    pattern: "retry".to_string(),
                    is_regex: false,
                },
                ParseRetryType::FetchNew,
            ))],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let config = SpiderConfig::default().with_retry(retry_config);
    let spider = spider.with_config(config);

    let scraper = Box::new(MockScraper::new(mock_responses));
    let crawler = Crawler::new(scraper);

    crawler.run(spider).await.unwrap();

    assert_eq!(*retry_count.read(), 3); // Initial + 2 retries with new content
}

#[tokio::test]
async fn test_crawler_storage_error_retry() {
    let retry_count = Arc::new(RwLock::new(0));
    let max_attempts = 3;
    let spider = TestSpider::new_with_storage_error(Arc::clone(&retry_count), max_attempts);

    let mock_responses = vec![MockResponse {
        status: 200,
        body: "test response".to_string(),
        delay: None,
    }];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::StorageError,
        CategoryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1),
            conditions: vec![RetryCondition::Parse(ParseRetryCondition::StorageError(
                StorageError::OperationError("test storage error".to_string()),
                ParseRetryType::SameContent,
            ))],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let config = SpiderConfig::default().with_retry(retry_config);
    let spider = spider.with_config(config);

    let scraper = Box::new(MockScraper::new(mock_responses));
    let crawler = Crawler::new(scraper);

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            crawler.run(spider).await.unwrap();

            assert_eq!(
                *retry_count.read(),
                max_attempts,
                "Expected {} attempts (initial + {} retries)",
                max_attempts,
                max_attempts - 1
            );
        })
        .await;
}

#[tokio::test]
async fn test_crawler_max_retries_limit() {
    let retry_count = Arc::new(RwLock::new(0));
    let spider = TestSpider::new(
        Arc::clone(&retry_count),
        RetryBehavior::RetryWithSame {
            max_attempts: 99,
            error: None,
        },
    );

    let mock_responses = vec![MockResponse {
        status: 200,
        body: "test response".to_string(),
        delay: None,
    }];

    let mut retry_config = RetryConfig::default();
    retry_config.categories.insert(
        RetryCategory::ParseError,
        CategoryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1),
            conditions: vec![RetryCondition::Parse(ParseRetryCondition::Content(
                ContentRetryCondition {
                    pattern: "retry".to_string(),
                    is_regex: false,
                },
                ParseRetryType::SameContent,
            ))],
            backoff_policy: BackoffPolicy::Constant,
        },
    );

    let config = SpiderConfig::default().with_retry(retry_config);
    let spider = spider.with_config(config);

    let scraper = Box::new(MockScraper::new(mock_responses));
    let crawler = Crawler::new(scraper);

    crawler.run(spider).await.unwrap();

    assert_eq!(*retry_count.read(), 6); // Initial + 1 retry (max reached)
}

#[tokio::test]
async fn test_crawler_no_retry() {
    let retry_count = Arc::new(RwLock::new(0));
    let spider = TestSpider::new(Arc::clone(&retry_count), RetryBehavior::NoRetry);

    let mock_responses = vec![MockResponse {
        status: 200,
        body: "test content".to_string(),
        delay: None,
    }];

    let config = SpiderConfig::default();
    let spider = spider.with_config(config);

    let scraper = Box::new(MockScraper::new(mock_responses));
    let crawler = Crawler::new(scraper);

    crawler.run(spider).await.unwrap();

    assert_eq!(
        *retry_count.read(),
        1,
        "Expected exactly one attempt with no retries"
    );
}
