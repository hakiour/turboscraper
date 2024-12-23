use crate::{http::HttpRequest, HttpResponse, ScraperResult};
use async_trait::async_trait;
use url::Url;

#[async_trait]
pub trait Parser: Send + Sync {
    async fn parse(
        &self,
        response: HttpResponse,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<HttpRequest>>;
}
