use super::{Response, Scraper};
use crate::errors::ScraperResult;
use async_trait::async_trait;
use chrono::Utc;
use reqwest::{header, Client};
use url::Url;

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Clone)]
pub struct HttpScraper {
    client: Client,
}

impl HttpScraper {
    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(DEFAULT_USER_AGENT),
        );

        Self {
            client: Client::builder().default_headers(headers).build().unwrap(),
        }
    }

    pub fn with_user_agent(user_agent: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(user_agent).unwrap(),
        );

        Self {
            client: Client::builder().default_headers(headers).build().unwrap(),
        }
    }
}

#[async_trait]
impl Scraper for HttpScraper {
    async fn fetch(&self, url: Url) -> ScraperResult<Response> {
        let response = self.client.get(url.clone()).send().await?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect();
        let body = response.text().await?;

        Ok(Response {
            url,
            status,
            headers,
            body,
            timestamp: Utc::now(),
        })
    }

    fn box_clone(&self) -> Box<dyn Scraper> {
        Box::new(self.clone())
    }
}
