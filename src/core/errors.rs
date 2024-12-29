use crate::{storage::base::StorageError, HttpRequest};
use thiserror::Error;

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
    ExtractionError(String),

    #[error("Middleware error: {0}")]
    MiddlewareError(String),

    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
}

pub type ScraperResult<T> = Result<T, (ScraperError, Box<HttpRequest>)>; //When we have an error, we want to return the error and the request we were processing when the error occurred
