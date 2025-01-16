use crate::{storage::base::StorageError, HttpRequest};
use thiserror::Error;
use url::Url;

use super::retry::RetryCategory;

#[derive(Error, Debug)]
pub enum ScraperError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("URL parsing error: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Extraction error: {0}")]
    ParsingError(String),

    #[error("Middleware error: {0}")]
    MiddlewareError(String),

    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),

    #[error("Maximum retries of {retry_count} reached for category {category:?} on url: {url}")]
    MaxRetriesReached {
        category: RetryCategory,
        retry_count: usize,
        url: Box<Url>,
    },
}

pub type ScraperResult<T> = Result<T, (ScraperError, Box<HttpRequest>)>;
