use crate::errors::ScraperResult;
use crate::scraper::Request;
use crate::scraper::Response;
use crate::storage::Storage;
use crate::Spider;
use async_trait::async_trait;
use log::{debug, info, trace};
use scraper::{Html, Selector};
use serde_json::json;
use url::Url;

pub struct ExampleSpider {
    storage: Storage,
    max_depth: usize,
}

impl ExampleSpider {
    pub fn new() -> ScraperResult<Self> {
        Ok(Self {
            storage: Storage::new("output")?,
            max_depth: 2,
        })
    }

    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }
}

#[async_trait]
impl Spider for ExampleSpider {
    fn name(&self) -> String {
        "example_spider".to_string()
    }

    fn start_urls(&self) -> Vec<Url> {
        vec![Url::parse("https://books.toscrape.com/").unwrap()]
    }

    fn max_depth(&self) -> usize {
        self.max_depth
    }

    async fn parse(
        &self,
        response: Response,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<Request>> {
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
                        callback: crate::spider::Callback::Parse,
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
