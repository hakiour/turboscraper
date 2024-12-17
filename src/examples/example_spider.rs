use crate::errors::ScraperResult;
use crate::scraper::Response;
use crate::storage::Storage;
use crate::{spider::Request, Spider};
use async_trait::async_trait;
use scraper::{Html, Selector};
use url::Url;

pub struct ExampleSpider {
    storage: Storage,
}

impl ExampleSpider {
    pub fn new() -> ScraperResult<Self> {
        Ok(Self {
            storage: Storage::new("output")?,
        })
    }
}

#[async_trait]
impl Spider for ExampleSpider {
    fn name(&self) -> String {
        "example_spider".to_string()
    }

    fn start_urls(&self) -> Vec<Url> {
        vec![Url::parse("https://www.rust-lang.org").unwrap()]
    }

    fn max_depth(&self) -> usize {
        2 // Only go 2 levels deep
    }

    async fn parse(
        &self,
        response: Response,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<Request>> {
        let saved_path = self.storage.save_response(&response)?;
        println!(
            "Saved response to: {} (depth: {})",
            saved_path.display(),
            depth
        );

        let document = Html::parse_document(&response.body);
        let selector = Selector::parse("a").unwrap();

        println!(
            "Parsing URL: {} (Status: {}, Depth: {})",
            url, response.status, depth
        );

        let mut requests = Vec::new();
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(new_url) = url.join(href) {
                    requests.push(Request {
                        url: new_url,
                        callback: crate::spider::Callback::Parse,
                        meta: None,
                        depth: depth + 1,
                    });
                }
            }
        }

        Ok(requests)
    }
}
