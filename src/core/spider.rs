use crate::{Request, Response, ScraperResult};
use async_trait::async_trait;
use url::Url;

#[derive(Debug, Clone, PartialEq)]
pub enum SpiderCallback {
    Bootstrap,       // For initial page
    ParseItem,       // For parsing detail pages (e.g., product pages)
    ParsePagination, // For handling pagination
    Custom(String),  // For custom parsing methods
}

#[derive(Debug)]
pub enum ParseResult {
    Continue(Vec<Request>), // Continue crawling with these requests
    Skip,                   // Skip this URL but continue crawling
    Stop,                   // Stop crawling
}

#[derive(Debug, Clone)]
pub struct SpiderResponse {
    pub response: Response,
    pub callback: SpiderCallback,
}

#[derive(Debug, Clone)]
pub struct SpiderConfig {
    pub max_depth: usize,
    pub max_concurrency: usize,
}

impl Default for SpiderConfig {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_concurrency: 10,
        }
    }
}

#[async_trait]
pub trait Spider {
    fn name(&self) -> String;
    fn start_urls(&self) -> Vec<Url>;
    fn config(&self) -> &SpiderConfig;
    
    async fn parse(
        &self,
        response: SpiderResponse,
        url: Url,
        depth: usize,
    ) -> ScraperResult<ParseResult>;

    fn get_initial_requests(&self) -> Vec<Request> {
        self.start_urls()
            .into_iter()
            .map(|url| Request::new(url, self.get_initial_callback(), 0))
            .collect()
    }

    fn get_initial_callback(&self) -> SpiderCallback {
        SpiderCallback::Bootstrap
    }

    fn allowed_domains(&self) -> Option<Vec<String>> {
        None
    }
}
