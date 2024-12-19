use crate::errors::ScraperResult;
use crate::scraper::Request;
use crate::scraper::Response;
use async_trait::async_trait;
use url::Url;

#[derive(Debug, Clone)]
pub enum Callback {
    Parse,
    Custom(String),
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
        response: Response,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<Request>>;

    fn allowed_domains(&self) -> Option<Vec<String>> {
        None
    }
}
