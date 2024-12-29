use crate::storage::base::StorageError;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ContentRetryCondition {
    pub pattern: String,
    pub is_regex: bool,
}

#[derive(Debug, Clone)]
pub enum RequestRetryCondition {
    StatusCode(u16),
    Content(ContentRetryCondition),
}

#[derive(Debug, Clone)]
pub enum ParseRetryType {
    SameContent, // Retry with the same response content
    FetchNew,    // Fetch the URL again and retry with new content
}

#[derive(Debug, Clone)]
pub enum ParseRetryCondition {
    Content(ContentRetryCondition, ParseRetryType),
    StorageError(StorageError, ParseRetryType),
}

#[derive(Debug, Clone, Copy)]
pub enum BackoffPolicy {
    Constant,
    Linear,
    Exponential { factor: f32 },
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum RetryCategory {
    RateLimit,      // 429, rate limiting messages
    ServerError,    // 500-599
    BotDetection,   // Bot detection, captchas
    NotFound,       // 404s that might be temporary
    Blacklisted,    // IP blocked messages
    Authentication, // 401, 403
    Custom(String), // Custom category
    StorageError,   // Storage-related errors
    ParseError,     // Parse-related errors
}

#[derive(Debug, Clone)]
pub enum RetryCondition {
    Request(RequestRetryCondition),
    Parse(ParseRetryCondition),
}

#[derive(Debug, Clone)]
pub struct CategoryConfig {
    pub max_retries: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_policy: BackoffPolicy,
    pub conditions: Vec<RetryCondition>,
}

#[derive(Debug, Clone)]
pub struct RetryState {
    pub counts: HashMap<RetryCategory, usize>,
    pub total_retries: usize,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub categories: HashMap<RetryCategory, CategoryConfig>,
    pub(crate) retry_states: Arc<RwLock<HashMap<String, RetryState>>>,
}
