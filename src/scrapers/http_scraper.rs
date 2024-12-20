use async_trait::async_trait;
use chrono::Utc;
use reqwest::{header, Client, Method};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use url::Url;

use crate::core::retry::RetryConfig;
use crate::http::ResponseType;
use crate::Response;
use crate::{ScraperResult, StatsTracker};

use super::Scraper;
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Clone)]
pub struct HttpScraper {
    client: Client,
    retry_config: RetryConfig,
    stats: Arc<RwLock<Arc<StatsTracker>>>,
}

impl Default for HttpScraper {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpScraper {
    pub fn new() -> Self {
        Self::with_config(RetryConfig::default())
    }

    pub fn with_config(retry_config: RetryConfig) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(DEFAULT_USER_AGENT),
        );

        Self {
            client: Client::builder().default_headers(headers).build().unwrap(),
            retry_config,
            stats: Arc::new(RwLock::new(Arc::new(StatsTracker::new()))),
        }
    }

    pub fn with_user_agent(user_agent: &str) -> Self {
        Self::with_user_agent_and_config(user_agent, RetryConfig::default())
    }

    pub fn with_user_agent_and_config(user_agent: &str, retry_config: RetryConfig) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(user_agent).unwrap(),
        );

        Self {
            client: Client::builder().default_headers(headers).build().unwrap(),
            retry_config,
            stats: Arc::new(RwLock::new(Arc::new(StatsTracker::new()))),
        }
    }

    pub async fn fetch_with_method(
        &self,
        method: Method,
        url: Url,
        body: Option<String>,
    ) -> ScraperResult<Response> {
        let start_time = Utc::now();
        let mut request = self.client.request(method.clone(), url.clone());

        if let Some(body) = body {
            request = request.body(body);
        }

        let response = request.send().await?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response.text().await?;
        let end_time = Utc::now();

        let headers: HashMap<String, String> = headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect();

        let meta = json!({
            "request": {
                "method": method.as_str(),
            },
            "response": {
                "elapsed": (end_time - start_time).num_milliseconds(),
                "content_length": body.len(),
            }
        });

        Ok(Response {
            url,
            status,
            headers,
            body,
            timestamp: start_time,
            retry_count: 0,
            retry_history: HashMap::new(),
            meta: Some(meta),
            response_type: ResponseType::Html,
        })
    }

    pub async fn get(&self, url: Url) -> ScraperResult<Response> {
        self.fetch_with_method(Method::GET, url, None).await
    }

    pub async fn post(&self, url: Url, body: String) -> ScraperResult<Response> {
        self.fetch_with_method(Method::POST, url, Some(body)).await
    }

    pub async fn put(&self, url: Url, body: String) -> ScraperResult<Response> {
        self.fetch_with_method(Method::PUT, url, Some(body)).await
    }

    pub async fn delete(&self, url: Url) -> ScraperResult<Response> {
        self.fetch_with_method(Method::DELETE, url, None).await
    }
}

#[async_trait]
impl Scraper for HttpScraper {
    async fn fetch_single(&self, url: Url) -> ScraperResult<Response> {
        self.get(url).await
    }

    fn box_clone(&self) -> Box<dyn Scraper> {
        Box::new(self.clone())
    }

    fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    fn stats(&self) -> &StatsTracker {
        static STATS: std::sync::OnceLock<StatsTracker> = std::sync::OnceLock::new();
        STATS.get_or_init(|| (*self.stats.read().unwrap()).as_ref().clone())
    }

    fn set_stats(&self, stats: Arc<StatsTracker>) {
        *self.stats.write().unwrap() = stats;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup() -> (HttpScraper, MockServer) {
        let server = MockServer::start().await;
        let scraper = HttpScraper::new();
        (scraper, server)
    }

    #[tokio::test]
    async fn test_get_request() {
        let (scraper, mock_server) = setup().await;

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
        let response = scraper.get(url).await.unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "Hello, World!");
    }

    #[tokio::test]
    async fn test_post_request() {
        let (scraper, mock_server) = setup().await;
        let body = json!({"key": "value"}).to_string();

        Mock::given(method("POST"))
            .and(path("/test"))
            .and(body_string(body.clone()))
            .respond_with(ResponseTemplate::new(201).set_body_string("{\"status\": \"created\"}"))
            .mount(&mock_server)
            .await;

        let url = Url::parse(&mock_server.uri())
            .unwrap()
            .join("/test")
            .unwrap();
        let response = scraper.post(url, body).await.unwrap();

        assert_eq!(response.status, 201);
        assert_eq!(response.body, "{\"status\": \"created\"}");
    }

    #[tokio::test]
    async fn test_error_handling() {
        let (scraper, mock_server) = setup().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let url = Url::parse(&mock_server.uri())
            .unwrap()
            .join("/error")
            .unwrap();
        let response = scraper.get(url).await.unwrap();

        assert_eq!(response.status, 404);
        assert_eq!(response.body, "Not Found");
    }

    #[tokio::test]
    async fn test_custom_user_agent() {
        let (_, mock_server) = setup().await;
        let custom_ua = "CustomBot/1.0";
        let scraper = HttpScraper::with_user_agent(custom_ua);

        Mock::given(method("GET"))
            .and(path("/"))
            .and(header("user-agent", custom_ua))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let url = Url::parse(&mock_server.uri()).unwrap();
        let response = scraper.get(url).await.unwrap();

        assert_eq!(response.status, 200);
    }
}
