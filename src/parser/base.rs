use crate::{Request, Response, ScraperResult};
use async_trait::async_trait;
use url::Url;

#[async_trait]
pub trait Parser: Send + Sync {
    async fn parse(
        &self,
        response: Response,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<Request>>;
}
