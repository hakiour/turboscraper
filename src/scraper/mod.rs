use crate::errors::ScraperResult;
use async_trait::async_trait;
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
}

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn fetch(&self, url: Url) -> ScraperResult<Response>;

    /// Clone the scraper to be used in different tasks
    fn box_clone(&self) -> Box<dyn Scraper>;
}

pub mod playwright;
pub mod reqwest;
