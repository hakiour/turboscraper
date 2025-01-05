use crate::core::retry::RetryCategory;
use crate::core::spider::{ParseResult, SpiderConfig, SpiderResponse};
use crate::core::SpiderCallback;
use crate::http::{HttpRequest, HttpResponse};
use crate::storage::{StorageCategory, StorageItem, StorageManager};
use crate::{ScraperResult, Spider};
use async_trait::async_trait;
use chrono::Utc;
use log::error;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use url::Url;

pub struct BookSpider {
    config: SpiderConfig,
    start_urls: Vec<Url>,
    storage_manager: StorageManager,
}

impl BookSpider {
    pub fn new(storage_manager: StorageManager) -> ScraperResult<Self> {
        Ok(Self {
            config: SpiderConfig::default(),
            start_urls: vec![Url::parse("https://books.toscrape.com/").unwrap()],
            storage_manager,
        })
    }

    fn parse_book_list(
        &self,
        response: HttpResponse,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<HttpRequest>> {
        let document = Html::parse_document(&response.body);
        let book_selector = Selector::parse("article.product_pod h3 a").unwrap();

        let mut requests = Vec::new();
        for element in document.select(&book_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(new_url) = url.join(href) {
                    let req = HttpRequest::new(new_url, SpiderCallback::ParseItem, depth + 1)
                        .with_meta(json!({
                            "parent_url": url.to_string(),
                            "title": element.text().collect::<String>(),
                            "depth": depth,
                        }))?;
                    requests.push(req);
                }
            }
        }

        Ok(requests)
    }

    fn next_page(
        &self,
        response: HttpResponse,
        url: Url,
        depth: usize,
    ) -> ScraperResult<Vec<HttpRequest>> {
        let document = Html::parse_document(&response.body);
        let next_page_selector = Selector::parse("li.next a").unwrap();
        let mut requests = Vec::new();

        if let Some(next_element) = document.select(&next_page_selector).next() {
            if let Some(href) = next_element.value().attr("href") {
                if let Ok(next_url) = url.join(href) {
                    requests.push(HttpRequest::new(
                        next_url,
                        SpiderCallback::ParsePagination,
                        depth,
                    ));
                }
            }
        }
        Ok(requests)
    }

    async fn parse_book(
        &self,
        response: HttpResponse,
        url: Url,
        depth: usize,
    ) -> ScraperResult<()> {
        let details = self.parse_book_details(&response.body);

        let item = StorageItem {
            url,
            timestamp: Utc::now(),
            data: details,
            metadata: Some(json!({
                "depth": depth,
                "parser": "book_details",
                "response": {
                    "status": response.status,
                    "headers": response.headers,
                }
            })),
            id: self.name(),
        };

        self.store_data(item, StorageCategory::Data, response.from_request)
            .await?;

        Ok(())
    }

    fn parse_book_details(&self, body: &str) -> Value {
        let doc = Html::parse_document(body);

        // Title
        let title_sel = Selector::parse("div.product_main h1").unwrap();
        let title = doc
            .select(&title_sel)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();

        // Price
        let price_sel = Selector::parse("p.price_color").unwrap();
        let price = doc
            .select(&price_sel)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();

        // Availability
        let avail_sel = Selector::parse("p.availability").unwrap();
        let availability = doc
            .select(&avail_sel)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();

        // UPC (from table)
        let upc_sel = Selector::parse("table.table tr:nth-of-type(1) td").unwrap();
        let upc = doc
            .select(&upc_sel)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();

        // Description
        let desc_sel = Selector::parse("#product_description ~ p").unwrap();
        let description = doc
            .select(&desc_sel)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();

        // Convert to JSON for easy handling
        json!({
            "title": title.trim(),
            "price": price.trim(),
            "availability": availability,
            "upc": upc.trim(),
            "description": description.trim(),
        })
    }
}

#[async_trait]
impl Spider for BookSpider {
    fn config(&self) -> &SpiderConfig {
        &self.config
    }
    fn set_config(&mut self, config: SpiderConfig) {
        self.config = config;
    }

    fn storage_manager(&self) -> &StorageManager {
        &self.storage_manager
    }

    fn name(&self) -> String {
        "book_spider".to_string()
    }

    fn start_requests(&self) -> Vec<HttpRequest> {
        self.start_urls
            .clone()
            .into_iter()
            .map(|url| HttpRequest::new(url, SpiderCallback::Bootstrap, 0))
            .collect()
    }

    /// Main `parse` entrypoint
    async fn parse(
        &self,
        spider_response: SpiderResponse,
        url: Url,
        depth: usize,
    ) -> ScraperResult<ParseResult> {
        match spider_response.callback {
            // Bootstrap or pagination pages list multiple books
            SpiderCallback::Bootstrap | SpiderCallback::ParsePagination => {
                let mut requests =
                    self.parse_book_list(spider_response.response.clone(), url.clone(), depth)?;
                let next_page_requests = self.next_page(spider_response.response, url, depth)?;
                requests.extend(next_page_requests);
                Ok(ParseResult::Continue(requests))
            }
            // A single item page
            SpiderCallback::ParseItem => {
                self.parse_book(spider_response.response, url, depth)
                    .await?;
                Ok(ParseResult::Skip)
            }
            // Any custom callback is currently unhandled
            SpiderCallback::Custom(ref name) => {
                error!("Unhandled custom callback: {}", name);
                Ok(ParseResult::Skip)
            }
        }
    }

    async fn handle_max_retries(
        &self,
        category: RetryCategory,
        request: Box<HttpRequest>,
    ) -> ScraperResult<()> {
        let error_item = StorageItem {
            url: request.url.clone(),
            timestamp: Utc::now(),
            data: json!({
                "error": format!("Max retries reached for category {:?}", category),
                "spider": self.name(),
                "request": *request,
            }),
            metadata: Some(json!({
                "error_type": "max_retries",
                "category": format!("{:?}", category),
            })),
            id: format!("{}_errors", self.name()),
        };

        self.store_data(error_item, StorageCategory::Error, request)
            .await
    }
}
