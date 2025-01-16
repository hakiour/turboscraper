use crate::core::retry::RetryCategory;
use chrono::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

use super::HttpRequest;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub url: Url,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub raw_body: Vec<u8>,
    pub decoded_body: String,
    pub timestamp: DateTime<Utc>,
    pub retry_count: usize,
    pub retry_history: HashMap<RetryCategory, usize>,
    pub meta: Option<Value>,
    pub response_type: ResponseType,
    pub from_request: Box<HttpRequest>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseType {
    Html,
    Json,
    Text,
    Binary,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentEncoding {
    Gzip,
    Deflate,
    Brotli,
    None,
}

impl HttpResponse {
    pub fn detect_content_type(
        &self,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> ResponseType {
        fn detect_content_type_from_body(body: &str) -> ResponseType {
            let trimmed_body = body.trim_start();
            if trimmed_body.starts_with('{') || trimmed_body.starts_with('[') {
                ResponseType::Json
            } else if trimmed_body.starts_with("<!DOCTYPE") || trimmed_body.starts_with("<html") {
                ResponseType::Html
            } else {
                ResponseType::Text
            }
        }

        headers
            .get("content-type")
            .filter(|content_type| !content_type.trim().is_empty())
            .map(|content_type| {
                if content_type.contains("text/html") {
                    ResponseType::Html
                } else if content_type.contains("application/json") {
                    ResponseType::Json
                } else if content_type.contains("text/") {
                    ResponseType::Text
                } else {
                    ResponseType::Binary
                }
            })
            .unwrap_or_else(|| detect_content_type_from_body(body))
    }

    pub fn get_content_encoding(&self) -> ContentEncoding {
        if let Some(encoding) = self.headers.get("content-encoding") {
            match encoding.to_lowercase().as_str() {
                "gzip" => ContentEncoding::Gzip,
                "deflate" => ContentEncoding::Deflate,
                "br" => ContentEncoding::Brotli,
                _ => ContentEncoding::None,
            }
        } else {
            ContentEncoding::None
        }
    }
}

impl std::fmt::Display for ResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseType::Html => write!(f, "html"),
            ResponseType::Json => write!(f, "json"),
            ResponseType::Text => write!(f, "text"),
            ResponseType::Binary => write!(f, "binary"),
        }
    }
}
