use crate::core::spider::SpiderCallback;
use crate::ScraperResult;
use serde_json::Value;
use url::Url;

#[derive(Debug, Clone)]
pub struct Request {
    pub url: Url,
    pub callback: SpiderCallback,
    pub meta: Option<Value>,
    pub depth: usize,
}

impl Request {
    pub fn new(url: Url, callback: SpiderCallback, depth: usize) -> Self {
        Self {
            url,
            callback,
            meta: None,
            depth,
        }
    }

    pub fn with_meta<T: serde::Serialize>(mut self, meta: T) -> ScraperResult<Self> {
        self.meta = Some(serde_json::to_value(meta)?);
        Ok(self)
    }
}
