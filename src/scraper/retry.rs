use regex::Regex;
use std::time::Duration;

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
pub struct RetryConfig {
    pub categories: std::collections::HashMap<RetryCategory, CategoryConfig>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        let mut categories = std::collections::HashMap::new();

        // Rate Limit Configuration
        categories.insert(
            RetryCategory::RateLimit,
            CategoryConfig {
                max_retries: 5,
                initial_delay: Duration::from_secs(5),
                max_delay: Duration::from_secs(300),
                conditions: vec![
                    RetryCondition::StatusCode(429),
                    RetryCondition::Content(ContentRetryCondition {
                        pattern: "rate limit|too many requests".to_string(),
                        is_regex: true,
                    }),
                ],
                ..Default::default()
            },
        );

        // Server Error Configuration
        categories.insert(
            RetryCategory::ServerError,
            CategoryConfig {
                max_retries: 3,
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(30),
                conditions: vec![
                    RetryCondition::StatusCode(500),
                    RetryCondition::StatusCode(502),
                    RetryCondition::StatusCode(503),
                    RetryCondition::StatusCode(504),
                ],
                ..Default::default()
            },
        );

        // Bot Detection Configuration
        categories.insert(
            RetryCategory::BotDetection,
            CategoryConfig {
                max_retries: 3,
                initial_delay: Duration::from_secs(10),
                max_delay: Duration::from_secs(600),
                conditions: vec![RetryCondition::Content(ContentRetryCondition {
                    pattern: "bot detected|captcha|verify human|automated".to_string(),
                    is_regex: true,
                })],
                ..Default::default()
            },
        );

        // Blacklisted Configuration
        categories.insert(
            RetryCategory::Blacklisted,
            CategoryConfig {
                max_retries: 5,
                initial_delay: Duration::from_secs(60),
                max_delay: Duration::from_secs(3600),
                conditions: vec![RetryCondition::Content(ContentRetryCondition {
                    pattern: "ip.*blocked|access denied|blacklisted".to_string(),
                    is_regex: true,
                })],
                ..Default::default()
            },
        );

        Self { categories }
    }
}

impl RetryConfig {
    pub fn should_retry(
        &self,
        status: u16,
        content: &str,
        retry_count: &mut std::collections::HashMap<RetryCategory, usize>,
    ) -> Option<(RetryCategory, Duration)> {
        for (category, config) in &self.categories {
            let current_retries = retry_count.get(category).copied().unwrap_or(0);
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
                    retry_count.insert(category.clone(), new_count);
                    let delay = config.calculate_delay(current_retries);
                    return Some((category.clone(), delay));
                }
            }
        }
        None
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
