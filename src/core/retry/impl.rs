use crate::ScraperError;

use super::types::*;
use super::utils::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

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

impl RetryConfig {
    pub fn should_retry_request(
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
                if let RetryCondition::Request(req_condition) = condition {
                    if retry_request_condition_should_apply(req_condition, status, content) {
                        let new_count = current_retries + 1;
                        state.counts.insert(category.clone(), new_count);
                        state.total_retries += 1;
                        let delay = calculate_delay(config, current_retries);
                        return Some((category.clone(), delay));
                    }
                }
            }
        }
        None
    }

    pub fn should_retry_parse(
        &self,
        url: &Url,
        error: &ScraperError,
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
                if let RetryCondition::Parse(parse_condition) = condition {
                    if retry_parse_condition_should_apply(parse_condition, error) {
                        let new_count = current_retries + 1;
                        state.counts.insert(category.clone(), new_count);
                        state.total_retries += 1;
                        let delay = calculate_delay(config, current_retries);
                        return Some((category.clone(), delay));
                    }
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
        calculate_delay(self, attempt)
    }
}
