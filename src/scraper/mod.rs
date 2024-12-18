mod response;
mod retry;
mod scraper;
pub mod http_scraper;
pub mod mock_scraper;

#[cfg(test)]
mod tests;

pub use response::Response;
pub use retry::{BackoffPolicy, RetryConfig, RetryCondition, ContentRetryCondition};
pub use scraper::Scraper;