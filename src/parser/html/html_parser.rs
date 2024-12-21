use crate::core::Callback;
use crate::storage::Storage;
use crate::{Parser, Request, Response, ScraperResult};
use async_trait::async_trait;
use log::{debug, info, trace};
use scraper::{Html, Selector};
use serde_json::json;
use url::Url;

pub struct HtmlParser {
    storage: Storage,
}

impl HtmlParser {
    pub fn new() -> ScraperResult<Self> {
        Ok(Self {
            storage: Storage::new("output")?,
        })
    }
}

#[async_trait]
impl Parser for HtmlParser {
    async fn parse(&self, response: Response, url: Url, depth: usize) -> ScraperResult<Vec<Request>> {
        let saved_path = self.storage.save_response(&response)?;
        debug!(
            "Saved response to: {} (depth: {})",
            saved_path.display(),
            depth
        );

        let document = Html::parse_document(&response.body);
        let selector = Selector::parse("a").unwrap();

        trace!("Parsing HTML content: {}", response.body);
        info!("Processing URL: {} at depth {}", url, depth);
        debug!("Response status: {}", response.status);

        let mut requests = Vec::new();
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(new_url) = url.join(href) {
                    let meta = json!({
                        "parent_url": url.to_string(),
                        "link_text": element.text().collect::<String>(),
                        "title": element.value().attr("title").unwrap_or_default(),
                        "parent_depth": depth,
                    });

                    let request = Request {
                        url: new_url,
                        callback: Callback::Parse,
                        meta: Some(meta),
                        depth: depth + 1,
                    };

                    requests.push(request);
                }
            }
        }

        Ok(requests)
    }
} 