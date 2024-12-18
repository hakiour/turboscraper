use std::time::Duration;
use regex::Regex;

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

impl Default for BackoffPolicy {
    fn default() -> Self {
        Self::Exponential { factor: 2.0 }
    }
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub retry_conditions: Vec<RetryCondition>,
    pub backoff_policy: BackoffPolicy,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            retry_conditions: vec![
                RetryCondition::StatusCode(408),
                RetryCondition::StatusCode(429),
                RetryCondition::StatusCode(500),
                RetryCondition::StatusCode(502),
                RetryCondition::StatusCode(503),
                RetryCondition::StatusCode(504),
                RetryCondition::Content(ContentRetryCondition {
                    pattern: "bot detected".to_string(),
                    is_regex: false,
                }),
            ],
            backoff_policy: BackoffPolicy::default(),
        }
    }
}

impl RetryConfig {
    pub fn should_retry(&self, status: u16, content: &str, retry_count: usize) -> bool {
        if retry_count >= self.max_retries {
            return false;
        }

        for condition in &self.retry_conditions {
            match condition {
                RetryCondition::StatusCode(code) if *code == status => return true,
                RetryCondition::Content(content_condition) => {
                    if content_condition.is_regex {
                        if let Ok(regex) = Regex::new(&content_condition.pattern) {
                            if regex.is_match(content) {
                                return true;
                            }
                        }
                    } else if content.to_lowercase().contains(&content_condition.pattern.to_lowercase()) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

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