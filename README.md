# TurboScraper

A high-performance, concurrent web scraping framework for Rust, powered by Tokio. TurboScraper provides a robust foundation for building scalable web scrapers with built-in support for retries, storage backends, and concurrent request handling.

## Features

- 🚀 **High Performance**: Built on Tokio for async I/O and concurrent request handling
- 🔄 **Smart Retries**: Configurable retry mechanisms for both HTTP requests and parsing failures
- 💾 **Multiple Storage Backends**: Support for MongoDB and filesystem storage
- 🎯 **Type-safe**: Leverages Rust's type system for reliable data extraction
- 🔧 **Configurable**: Extensive configuration options for crawling behavior
- 🛡️ **Error Handling**: Comprehensive error handling and reporting
- 📊 **Statistics**: Built-in request statistics and performance monitoring

## Quick Start

Add TurboScraper to your `Cargo.toml`:
```toml
[dependencies]
turboscraper = { version = "0.1.1" }
```

### Basic Spider Example

Here's a complete example of a spider that scrapes book information from a book catalog:

```rust
use turboscraper::prelude::*;
use async_trait::async_trait;
use scraper::{Html, Selector};
use serde_json::json;
use chrono::Utc;

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

    // Parse the book listing page to find book detail pages
    fn parse_book_list(&self, response: &HttpResponse) -> ScraperResult<Vec<HttpRequest>> {
        let document = Html::parse_document(&response.decoded_body);
        let book_selector = Selector::parse("article.product_pod h3 a").unwrap();
        let url = response.from_request.url.clone();
        let depth = response.from_request.depth;

        let mut requests = Vec::new();
        for element in document.select(&book_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(new_url) = url.join(href) {
                    let req = HttpRequest::new(new_url, SpiderCallback::ParseItem, depth + 1)
                        .with_meta(json!({
                            "parent_url": url.to_string(),
                            "title": element.text().collect::<String>(),
                        }))?;
                    requests.push(req);
                }
            }
        }
        Ok(requests)
    }

    // Find and follow pagination links
    fn next_page(&self, response: &HttpResponse) -> ScraperResult<Vec<HttpRequest>> {
        let document = Html::parse_document(&response.decoded_body);
        let next_selector = Selector::parse("li.next a").unwrap();
        let url = response.from_request.url.clone();
        let depth = response.from_request.depth;

        let mut requests = Vec::new();
        if let Some(next_element) = document.select(&next_selector).next() {
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

    // Extract detailed book information
    fn parse_book_details(&self, body: &str) -> Value {
        let doc = Html::parse_document(body);

        let title = doc.select(&Selector::parse("div.product_main h1").unwrap())
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();

        let price = doc.select(&Selector::parse("p.price_color").unwrap())
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();

        let availability = doc.select(&Selector::parse("p.availability").unwrap())
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();

        json!({
            "title": title.trim(),
            "price": price.trim(),
            "availability": availability,
            "extracted_at": Utc::now(),
        })
    }
}

#[async_trait]
impl Spider for BookSpider {
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

    fn parse(&self, spider_response: &SpiderResponse) -> ScraperResult<(ParseResult, ParsedData)> {
        match spider_response.callback {
            SpiderCallback::Bootstrap | SpiderCallback::ParsePagination => {
                let mut requests = self.parse_book_list(&spider_response.response)?;
                let next_page_requests = self.next_page(&spider_response.response)?;
                requests.extend(next_page_requests);
                Ok((ParseResult::Continue(requests), ParsedData::Empty))
            }
            SpiderCallback::ParseItem => {
                let details = self.parse_book_details(&spider_response.response.decoded_body);
                Ok((ParseResult::Skip, ParsedData::Item(details)))
            }
            SpiderCallback::Custom(ref name) => {
                log::error!("Unhandled custom callback: {}", name);
                Ok((ParseResult::Skip, ParsedData::Empty))
            }
        }
    }

    async fn persist_extracted_data(
        &self,
        data: ParsedData,
        response: &SpiderResponse,
    ) -> ScraperResult<()> {
        if let ParsedData::Item(details) = data {
            let item = StorageItem {
                url: response.response.from_request.url.clone(),
                timestamp: Utc::now(),
                data: details,
                metadata: Some(json!({
                    "depth": response.response.from_request.depth,
                    "parser": "book_details",
                    "response": {
                        "status": response.response.status,
                        "headers": response.response.headers,
                    }
                })),
                id: self.name(),
            };

            self.store_data(
                item,
                StorageCategory::Data,
                response.response.from_request.clone(),
            ).await?;
        }
        Ok(())
    }
}

### Running the Spider

```rust
use turboscraper::storage::StorageManager;

#[tokio::main]
async fn main() -> ScraperResult<()> {
    // Initialize storage manager with MongoDB
    let storage_config = StorageConfig::MongoDB {
        uri: "mongodb://localhost:27017",
        database: "book_scraper",
        collection: "books",
    };
    let storage_manager = StorageManager::new(storage_config);

    // Create and configure spider with reasonable defaults
    let spider = BookSpider::new(storage_manager)?;
    let config = SpiderConfig::default()
        .with_depth(2)
        .with_concurrency(10)
        .with_request_delay(Duration::from_millis(500))
        .with_respect_robots_txt(true)
        .with_user_agent("BookSpider/1.0 (+https://turboscraper.org/bot)");
    
    spider.set_config(config);

    // Create crawler and run spider
    let crawler = Crawler::new();
    crawler.run(spider).await?;
    Ok(())
}
```

## Advanced Features

### Storage System

TurboScraper implements a flexible storage system with categories:

```rust
async fn persist_extracted_data(
    &self,
    data: ParsedData,
    response: &SpiderResponse,
) -> ScraperResult<()> {
    if let ParsedData::Item(details) = data {
        let item = StorageItem {
            url: response.response.from_request.url.clone(),
            timestamp: Utc::now(),
            data: details,
            metadata: Some(json!({
                "depth": response.response.from_request.depth,
                "parser": "book_details"
            })),
            id: self.name(),
        };

        self.store_data(
            item,
            StorageCategory::Data,
            response.response.from_request.clone(),
        ).await?;
    }
    Ok(())
}
```

### Storage Backends

TurboScraper supports multiple storage backends:

- **MongoDB**: For scalable document storage
- **Filesystem**: For local file storage
- **Kafka**: For streaming data to Kafka topics
- **Custom**: Implement the `StorageBackend` trait for custom storage solutions

### Error Handling

Comprehensive error handling with retry mechanisms:

```rust
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
```

## Best Practices

1. **Respect Robots.txt**: Always check and respect website crawling policies
2. **Rate Limiting**: Use appropriate delays between requests
3. **Error Handling**: Implement proper error handling and retries
4. **Data Validation**: Validate scraped data before storage
5. **Resource Management**: Monitor memory and connection usage

## Contributing

Contributions are welcome! Please feel free to submit pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE file](LICENSE) for details.
