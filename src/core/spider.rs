use crate::{Request, Response, ScraperResult};
use async_trait::async_trait;
use url::Url;

#[derive(Debug, Clone, PartialEq)]
pub enum SpiderCallback {
    ParseList,       // For parsing list pages (e.g., category pages)
    ParseItem,       // For parsing detail pages (e.g., product pages)
    ParsePagination, // For handling pagination
    Custom(String),  // For custom parsing methods
}

#[derive(Debug, Clone)]
pub struct SpiderResponse {
    pub response: Response,
    pub callback: SpiderCallback,
}

#[async_trait]
pub trait Spider {
    fn name(&self) -> String;
    fn start_urls(&self) -> Vec<Url>;
    fn max_depth(&self) -> usize {
        2
    }

    async fn parse(
        &self,
        response: SpiderResponse,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<Request>>;

    fn allowed_domains(&self) -> Option<Vec<String>> {
        None
    }
}
