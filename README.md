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
turboscraper = { path = "path/to/turboscraper" }
```

### Basic Spider Example

Here's a simple spider that scrapes book information:
```rust
use turboscraper::prelude::*;

pub struct BookSpider {
    config: SpiderConfig,
    storage: Box<dyn StorageBackend>,
    storage_config: Box<dyn StorageConfig>,
}

#[async_trait]
impl Spider for BookSpider {
    fn name(&self) -> String {
        "book_spider".to_string()
    }

    fn start_urls(&self) -> Vec<Url> {
        vec![Url::parse("https://books.toscrape.com/").unwrap()]
    }

    async fn parse(
        &self,
        response: SpiderResponse,
        url: Url,
        depth: usize,
    ) -> ScraperResult<ParseResult> {
        match response.callback {
            SpiderCallback::Bootstrap => {
                // Parse book list and return new requests
                let new_requests = parse_book_list(&response.body)?;
                Ok(ParseResult::Continue(new_requests))
            }
            SpiderCallback::ParseItem => {
                // Parse and store book details
                self.parse_book_details(response).await?;
                Ok(ParseResult::Skip)
            }
            _ => Ok(ParseResult::Skip),
        }
    }
}
```

### Running the Spider

```rust
use turboscraper::storage::factory::{create_storage, StorageType};

#[tokio::main]
async fn main() -> ScraperResult<()> {
    // Initialize storage
    let storage = create_storage(StorageType::Mongo {
        connection_string: "mongodb://localhost:27017".to_string(),
        database: "books".to_string(),
    }).await?;

    // Create and configure spider
    let spider = BookSpider::new(storage).await?;
    let config = SpiderConfig::default()
        .with_depth(2)
        .with_concurrency(10);
    let spider = spider.with_config(config);

    // Create crawler and run spider
    let scraper = Box::new(HttpScraper::new());
    let crawler = Crawler::new(scraper);
    crawler.run(spider).await?;
    Ok(())
}
```

## Advanced Features

### Retry Configuration

TurboScraper supports sophisticated retry mechanisms:
```rust
let mut retry_config = RetryConfig::default();
retry_config.categories.insert(
    RetryCategory::HttpError,
    CategoryConfig {
        max_retries: 3,
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(60),
        conditions: vec![
            RetryCondition::Request(RequestRetryCondition::StatusCode(429)),
        ],
        backoff_policy: BackoffPolicy::Exponential { factor: 2.0 },
    },
);
```

### Storage Backends

TurboScraper supports multiple storage backends:

- **MongoDB**: For scalable document storage
- **Filesystem**: For local file storage
- **Custom**: Implement the `StorageBackend` trait for custom storage solutions

### Error Handling

Comprehensive error handling with custom error types:
```rust
match result {
    Ok(ParseResult::Continue(requests)) => // Handle new requests
    Ok(ParseResult::RetryWithSameContent(response)) => // Retry parsing
    Err(ScraperError::StorageError(e)) => // Handle storage errors
    Err(ScraperError::HttpError(e)) => // Handle HTTP errors
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

This project is licensed under the MIT License - see the LICENSE file for details.
