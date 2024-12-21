use crate::core::retry::RetryCategory;
use chrono::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone)]
pub struct Response {
    pub url: Url,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub timestamp: DateTime<Utc>,
    pub retry_count: usize,
    pub retry_history: HashMap<RetryCategory, usize>,
    pub meta: Option<Value>,
    pub response_type: ResponseType,
}

#[derive(Debug, Clone)]
pub enum ResponseType {
    Html,
    Json,
    Text,
}

impl std::fmt::Display for ResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseType::Html => write!(f, "html"),
            ResponseType::Json => write!(f, "json"),
            ResponseType::Text => write!(f, "text"),
        }
    }
}
