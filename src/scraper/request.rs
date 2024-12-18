use crate::errors::ScraperResult;
use crate::spider::Callback;
use serde_json::Value;
use url::Url;

#[derive(Debug, Clone)]
pub struct Request {
    pub url: Url,
    pub callback: Callback,
    pub meta: Option<Value>,
    pub depth: usize,
}

impl Request {
    pub fn with_meta<T: serde::Serialize>(mut self, meta: T) -> ScraperResult<Self> {
        self.meta = Some(serde_json::to_value(meta)?);
        Ok(self)
    }
}
