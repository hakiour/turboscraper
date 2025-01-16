use async_trait::async_trait;
use chrono::Utc;
use reqwest::{header, Client, ClientBuilder};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use super::Scraper;
use crate::core::spider::SpiderConfig;
use crate::http::request::HttpRequest;
use crate::http::response::ResponseType;
use crate::HttpResponse;
use crate::{ScraperError, ScraperResult, StatsTracker};

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug, Error)]
pub enum HttpScraperError {
    #[error("HTTP client error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Invalid header name: {0}")]
    InvalidHeaderName(#[from] header::InvalidHeaderName),
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] header::InvalidHeaderValue),
    #[error("Failed to decode response body: {0}")]
    DecodingError(String),
}

impl From<HttpScraperError> for ScraperError {
    fn from(err: HttpScraperError) -> Self {
        ScraperError::ParsingError(err.to_string())
    }
}

#[derive(Clone)]
pub struct HttpScraper {
    client: Client,
    stats: Arc<StatsTracker>,
}

impl Default for HttpScraper {
    fn default() -> Self {
        Self::new().expect("Failed to create default HttpScraper")
    }
}

impl HttpScraper {
    pub fn new() -> Result<Self, HttpScraperError> {
        let client = ClientBuilder::new()
            .user_agent(DEFAULT_USER_AGENT)
            .build()?;

        Ok(Self {
            client,
            stats: Arc::new(StatsTracker::new()),
        })
    }

    pub fn with_headers(mut self, headers: Vec<(&str, &str)>) -> Result<Self, HttpScraperError> {
        let mut header_map = header::HeaderMap::new();
        header_map.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(DEFAULT_USER_AGENT),
        );

        for (key, value) in headers {
            let name = header::HeaderName::from_bytes(key.as_bytes())?;
            let value = header::HeaderValue::from_str(value)?;
            header_map.insert(name, value);
        }

        self.client = ClientBuilder::new().default_headers(header_map).build()?;

        Ok(self)
    }

    fn extract_headers(response: &reqwest::Response) -> HashMap<String, String> {
        response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|val| (k.to_string(), val.to_string())))
            .collect()
    }

    fn detect_content_type(headers: &HashMap<String, String>, body: &str) -> ResponseType {
        if let Some(content_type) = headers.get("content-type") {
            if content_type.contains("text/html") {
                ResponseType::Html
            } else if content_type.contains("application/json") {
                ResponseType::Json
            } else if content_type.contains("text/") {
                ResponseType::Text
            } else {
                ResponseType::Binary
            }
        } else {
            // Try to detect content type from body
            if body.trim_start().starts_with('{') || body.trim_start().starts_with('[') {
                ResponseType::Json
            } else if body.trim_start().starts_with("<!DOCTYPE")
                || body.trim_start().starts_with("<html")
            {
                ResponseType::Html
            } else {
                ResponseType::Text
            }
        }
    }
}

#[async_trait]
impl Scraper for HttpScraper {
    async fn fetch_single(
        &self,
        request: HttpRequest,
        config: &SpiderConfig,
    ) -> ScraperResult<HttpResponse> {
        let method = request.method.clone();
        let from_request = request.clone();
        let mut req = self.client.request(method.clone(), request.url.clone());

        // Apply spider config headers
        for (key, value) in &config.headers {
            req = req.header(key, value);
        }

        // Apply request-specific headers
        for (key, value) in &request.headers {
            req = req.header(key, value);
        }

        if let Some(body) = request.body.clone() {
            req = req.body(body);
        }

        let start_time = Utc::now();
        let request_for_error = request.clone();
        let response = req.send().await.map_err(|e| {
            (
                ScraperError::from(HttpScraperError::HttpError(e)),
                Box::new(request_for_error),
            )
        })?;

        let status = response.status().as_u16();
        let headers = Self::extract_headers(&response);

        // Get raw bytes and decoded text
        let raw_body = response.bytes().await.map_err(|e| {
            (
                ScraperError::from(HttpScraperError::HttpError(e)),
                Box::new(request.clone()),
            )
        })?;

        let decoded_body = String::from_utf8(raw_body.to_vec()).map_err(|e| {
            (
                ScraperError::from(HttpScraperError::DecodingError(e.to_string())),
                Box::new(request.clone()),
            )
        })?;

        let end_time = Utc::now();

        let meta = json!({
            "request": {
                "method": method.as_str(),
            },
            "response": {
                "elapsed": (end_time - start_time).num_milliseconds(),
                "content_length": raw_body.len(),
                "encoding": headers.get("content-encoding").cloned().unwrap_or_default(),
            }
        });

        let response_type = Self::detect_content_type(&headers, &decoded_body);

        Ok(HttpResponse {
            url: request.url,
            status,
            headers,
            raw_body: raw_body.to_vec(),
            decoded_body,
            timestamp: start_time,
            retry_count: 0,
            retry_history: HashMap::new(),
            meta: Some(meta),
            response_type,
            from_request: Box::new(from_request),
        })
    }

    fn box_clone(&self) -> Box<dyn Scraper> {
        Box::new(self.clone())
    }

    fn stats(&self) -> &StatsTracker {
        &self.stats
    }

    fn set_stats(&mut self, stats: Arc<StatsTracker>) {
        self.stats = stats;
    }
}

#[cfg(test)]
mod tests {
    use crate::core::SpiderCallback;

    use super::*;
    use reqwest::Method;
    use url::Url;
    use wiremock::matchers::{body_string, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup() -> Result<(HttpScraper, MockServer), HttpScraperError> {
        let server = MockServer::start().await;
        let scraper = HttpScraper::new()?;
        Ok((scraper, server))
    }

    #[tokio::test]
    async fn test_get_request() {
        let (scraper, mock_server) = setup().await.unwrap();

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("Hello, World!")
                    .insert_header("content-type", "text/plain"),
            )
            .mount(&mock_server)
            .await;

        let url = Url::parse(&mock_server.uri())
            .unwrap()
            .join("/test")
            .unwrap();
        let response = scraper
            .fetch(
                HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
                &SpiderConfig::default(),
            )
            .await
            .unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.decoded_body, "Hello, World!");
        assert_eq!(response.response_type, ResponseType::Text);
    }

    #[tokio::test]
    async fn test_post_request() {
        let (scraper, mock_server) = setup().await.unwrap();
        let body = json!({"key": "value"}).to_string();

        Mock::given(method("POST"))
            .and(path("/test"))
            .and(body_string(body.clone()))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(json!({"status": "created"}))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        let url = Url::parse(&mock_server.uri())
            .unwrap()
            .join("/test")
            .unwrap();

        let request = HttpRequest::new(url, SpiderCallback::Bootstrap, 0)
            .with_method(Method::POST)
            .with_body(body);
        let response = scraper
            .fetch(request, &SpiderConfig::default())
            .await
            .unwrap();

        assert_eq!(response.status, 201);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response.decoded_body).unwrap(),
            json!({"status": "created"})
        );
        assert_eq!(response.response_type, ResponseType::Json);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let (scraper, mock_server) = setup().await.unwrap();

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let url = Url::parse(&mock_server.uri())
            .unwrap()
            .join("/error")
            .unwrap();
        let response = scraper
            .fetch(
                HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
                &SpiderConfig::default(),
            )
            .await
            .unwrap();

        assert_eq!(response.status, 404);
        assert_eq!(response.decoded_body, "Not Found");
        assert_eq!(response.response_type, ResponseType::Text);
    }

    #[tokio::test]
    async fn test_custom_headers() {
        let (scraper, mock_server) = setup().await.unwrap();
        let custom_ua = "CustomBot/1.0";
        let scraper = scraper
            .with_headers(vec![("user-agent", custom_ua)])
            .unwrap();

        Mock::given(method("GET"))
            .and(path("/"))
            .and(header("user-agent", custom_ua))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let url = Url::parse(&mock_server.uri()).unwrap();
        let response = scraper
            .fetch(
                HttpRequest::new(url, SpiderCallback::Bootstrap, 0),
                &SpiderConfig::default(),
            )
            .await
            .unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.decoded_body, "ok");
    }

    #[tokio::test]
    async fn test_invalid_headers() {
        let scraper = HttpScraper::new().unwrap();
        let result = scraper.with_headers(vec![("invalid\0header", "value")]);
        assert!(result.is_err());
    }
}
