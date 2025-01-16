use crate::{storage::base::StorageError, ScraperError};

use super::types::*;
use regex::Regex;
use std::time::Duration;

pub fn retry_request_condition_should_apply(
    condition: &RequestRetryCondition,
    status: u16,
    content: &str,
) -> bool {
    match condition {
        RequestRetryCondition::StatusCode(code) => *code == status,
        RequestRetryCondition::Content(content_condition) => {
            check_content_condition(content_condition, content)
        }
    }
}

pub fn retry_parse_condition_should_apply(
    condition: &ParseRetryCondition,
    error: &ScraperError,
) -> bool {
    match condition {
        ParseRetryCondition::Content(content_condition, _) => {
            if let ScraperError::ParsingError(msg) = error {
                check_content_condition(content_condition, msg)
            } else {
                false
            }
        }
        ParseRetryCondition::StorageError(expected_error, _) => {
            if let ScraperError::StorageError(actual_error) = error {
                matches!(
                    (expected_error, actual_error),
                    (
                        StorageError::ConnectionError(_),
                        StorageError::ConnectionError(_)
                    ) | (
                        StorageError::OperationError(_),
                        StorageError::OperationError(_)
                    ) | (
                        StorageError::SerializationError(_),
                        StorageError::SerializationError(_)
                    )
                )
            } else {
                false
            }
        }
        ParseRetryCondition::ErrorWhileParsing(_) => {
            if let ScraperError::ParsingError(_) = error {
                true
            } else {
                false
            }
        }
    }
}

fn check_content_condition(condition: &ContentRetryCondition, content: &str) -> bool {
    if condition.is_regex {
        Regex::new(&condition.pattern)
            .map(|re| re.is_match(content))
            .unwrap_or(false)
    } else {
        content
            .to_lowercase()
            .contains(&condition.pattern.to_lowercase())
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
