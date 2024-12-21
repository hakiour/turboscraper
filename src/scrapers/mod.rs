pub mod http_scraper;
pub mod mock_scraper;

mod scraper;
pub use scraper::Scraper;
pub use http_scraper::HttpScraper;
pub use mock_scraper::MockScraper;
