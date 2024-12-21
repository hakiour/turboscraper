use parking_lot::RwLock;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone)]
pub struct ContentRetryCondition {
    pub pattern: String,
    pub is_regex: bool,
}

#[derive(Debug, Clone)]
pub enum RetryCondition {
    StatusCode(u16),
    Content(ContentRetryCondition),
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
}

#[derive(Debug, Clone)]
pub struct CategoryConfig {
    pub max_retries: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_policy: BackoffPolicy,
    pub conditions: Vec<RetryCondition>,
}

impl Default for CategoryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_policy: BackoffPolicy::Exponential { factor: 2.0 },
            conditions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryState {
    pub counts: HashMap<RetryCategory, usize>,
    pub total_retries: usize,
}

impl Default for RetryState {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryState {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            total_retries: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub categories: HashMap<RetryCategory, CategoryConfig>,
    retry_states: Arc<RwLock<HashMap<String, RetryState>>>,
}

impl RetryConfig {
    pub fn should_retry(
        &self,
        url: &Url,
        status: u16,
        content: &str,
    ) -> Option<(RetryCategory, Duration)> {
        let url_str = url.to_string();
        let mut states = self.retry_states.write();
        let state = states.entry(url_str).or_default();

        for (category, config) in &self.categories {
            let current_retries = state.counts.get(category).copied().unwrap_or(0);
            if current_retries >= config.max_retries {
                continue;
            }

            for condition in &config.conditions {
                let matches = match condition {
                    RetryCondition::StatusCode(code) => *code == status,
                    RetryCondition::Content(content_condition) => {
                        if content_condition.is_regex {
                            Regex::new(&content_condition.pattern)
                                .map(|re| re.is_match(content))
                                .unwrap_or(false)
                        } else {
                            content
                                .to_lowercase()
                                .contains(&content_condition.pattern.to_lowercase())
                        }
                    }
                };

                if matches {
                    let new_count = current_retries + 1;
                    state.counts.insert(category.clone(), new_count);
                    state.total_retries += 1;
                    let delay = config.calculate_delay(current_retries);
                    return Some((category.clone(), delay));
                }
            }
        }
        None
    }

    pub fn get_retry_state(&self, url: &Url) -> RetryState {
        self.retry_states
            .read()
            .get(&url.to_string())
            .cloned()
            .unwrap_or_else(RetryState::new)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            categories: Default::default(),
            retry_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CategoryConfig {
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let delay = match self.backoff_policy {
            BackoffPolicy::Constant => self.initial_delay,
            BackoffPolicy::Linear => self.initial_delay.mul_f32(attempt as f32),
            BackoffPolicy::Exponential { factor } => {
                self.initial_delay.mul_f32(factor.powi(attempt as i32))
            }
        };

        std::cmp::min(delay, self.max_delay)
    }
}
