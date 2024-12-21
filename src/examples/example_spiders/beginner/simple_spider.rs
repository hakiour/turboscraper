use crate::core::spider::{ParseResult, SpiderResponse};
use crate::core::SpiderCallback;
use crate::storage::Storage;
use crate::{Request, Response, ScraperResult, Spider};
use async_trait::async_trait;
use log::{debug, error, info};
use scraper::{Html, Selector};
use serde_json::json;
use url::Url;

pub struct BookSpider {
    max_depth: usize,
    start_urls: Vec<Url>,
    storage: Storage,
}

impl BookSpider {
    pub fn new() -> ScraperResult<Self> {
        Ok(Self {
            max_depth: 2,
            start_urls: vec![Url::parse("https://books.toscrape.com/").unwrap()],
            storage: Storage::new("output")?,
        })
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    fn parse_book_list(
        &self,
        response: Response,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<Request>> {
        let document = Html::parse_document(&response.body.clone());

        // Parse book links
        let book_selector = Selector::parse("article.product_pod h3 a").unwrap();

        let mut requests = Vec::new();

        // Extract book details
        for element in document.select(&book_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(new_url) = url.join(href) {
                    let request = Request::new(new_url, SpiderCallback::ParseItem, depth + 1)
                        .with_meta(json!({
                            "parent_url": url.to_string(),
                            "title": element.text().collect::<String>(),
                            "depth": depth,
                        }))?;
                    requests.push(request);
                }
            }
        }

        Ok(requests)
    }

    fn next_page(
        &self,
        response: Response,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<Request>> {
        let document = Html::parse_document(&response.body);

        // Parse book links
        let next_page_selector = Selector::parse("li.next a").unwrap();

        let mut requests = Vec::new();

        // Handle pagination
        if let Some(next_element) = document.select(&next_page_selector).next() {
            if let Some(href) = next_element.value().attr("href") {
                if let Ok(next_url) = url.join(href) {
                    requests.push(Request::new(
                        next_url,
                        SpiderCallback::ParsePagination,
                        depth, // Same depth for pagination
                    ));
                }
            }
        }

        Ok(requests)
    }

    async fn parse_book(
        &self,
        response: Response,
        _url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<Request>> {
        let saved_path = self.storage.save_response(&response)?;
        debug!(
            "Saved book details to: {} (depth: {})",
            saved_path.display(),
            depth
        );

        let _document = Html::parse_document(&response.body);

        // Here you would extract specific book details
        // For now, we'll just return empty vec as this is a leaf node
        Ok(Vec::new())
    }
}

#[async_trait]
impl Spider for BookSpider {
    fn name(&self) -> String {
        "book_spider".to_string()
    }

    fn start_urls(&self) -> Vec<Url> {
        self.start_urls.clone()
    }

    fn max_depth(&self) -> usize {
        self.max_depth
    }

    async fn parse(&self, spider_response: SpiderResponse, url: Url, depth: usize) -> ScraperResult<ParseResult> {
        match spider_response.callback {
            SpiderCallback::Bootstrap | SpiderCallback::ParsePagination => {
                // Parse the book list
                let mut requests = self.parse_book_list(spider_response.response.clone(), url.clone(), depth)?;

                // Get the next page
                let next_page_requests = self.next_page(spider_response.response, url, depth)?;
                requests.extend(next_page_requests);

                Ok(ParseResult::Continue(requests))
            }
            SpiderCallback::ParseItem => {
                self.parse_book(spider_response.response, url, depth).await?;
                Ok(ParseResult::Skip)  // No more URLs to crawl from book pages
            }
            SpiderCallback::Custom(ref name) => {
                error!("Unhandled custom callback: {}", name);
                Ok(ParseResult::Skip)
            }
        }
    }
}
