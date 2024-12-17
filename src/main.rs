use turboscraper::examples::example_spider::ExampleSpider;
use turboscraper::scraper::reqwest::HttpScraper;
use turboscraper::{errors::ScraperResult, Crawler};

#[actix_rt::main]
async fn main() -> ScraperResult<()> {
    env_logger::init();

    let scraper = Box::new(HttpScraper::new());
    let crawler = Crawler::new(scraper, 30);
    let spider = ExampleSpider::new()?;

    crawler.run(spider).await?;

    Ok(())
}
