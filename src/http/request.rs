use reqwest::Method;
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

use crate::core::SpiderCallback;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: Url,
    pub callback: SpiderCallback,
    pub meta: Option<Value>,
    pub depth: usize, // Tracks the actual depth of the request
    pub method: Method,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl HttpRequest {
    pub fn new(url: Url, callback: SpiderCallback, depth: usize) -> Self {
        Self {
            url,
            callback,
            meta: None,
            depth,
            method: Method::GET,
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn with_method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_body<T: Into<String>>(mut self, body: T) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn with_meta<T: serde::Serialize>(mut self, meta: T) -> crate::ScraperResult<Self> {
        self.meta = Some(serde_json::to_value(meta).unwrap());
        Ok(self)
    }
}
