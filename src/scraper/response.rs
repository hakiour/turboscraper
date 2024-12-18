use chrono::prelude::*;
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
} 