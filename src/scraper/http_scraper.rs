use super::{Response, RetryConfig, Scraper};
use crate::errors::ScraperResult;
use async_trait::async_trait;
use chrono::Utc;
use reqwest::{header, Client};
use url::Url;

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Clone)]
pub struct HttpScraper {
    client: Client,
    retry_config: RetryConfig,
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
        }
    }
}

#[async_trait]
impl Scraper for HttpScraper {
    async fn fetch_single(&self, url: Url) -> ScraperResult<Response> {
        let response = self.client.get(url.clone()).send().await?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response.text().await?;

        let headers = headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect();

        Ok(Response {
            url,
            status,
            headers,
            body,
            timestamp: Utc::now(),
            retry_count: 0,
        })
    }

    fn box_clone(&self) -> Box<dyn Scraper> {
        Box::new(self.clone())
    }

    fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }
}
