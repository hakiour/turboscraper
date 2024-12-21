use super::types::*;
use regex::Regex;
use std::time::Duration;

pub fn check_condition(condition: &RetryCondition, status: u16, content: &str) -> bool {
    match condition {
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
    }
}

pub fn calculate_delay(config: &CategoryConfig, attempt: usize) -> Duration {
    if attempt == 0 {
        return config.initial_delay;
    }

    let delay = match config.backoff_policy {
        BackoffPolicy::Constant => config.initial_delay,
        BackoffPolicy::Linear => config.initial_delay.mul_f32(attempt as f32),
        BackoffPolicy::Exponential { factor } => {
            config.initial_delay.mul_f32(factor.powi(attempt as i32))
        }
    };

    std::cmp::min(delay, config.max_delay)
}
