
use crate::parser::HtmlParser;
use crate::parser::Parser;
use crate::ScraperResult;
use crate::Spider;
use url::Url;

pub struct BookSpider { 
    max_depth: usize,
    start_urls: Vec<Url>,
    parser: HtmlParser,
}

impl BookSpider {
    pub fn new() -> ScraperResult<Self> {
        Ok(Self {
            max_depth: 2,
            start_urls: vec![Url::parse("https://books.toscrape.com/").unwrap()],
            parser: HtmlParser::new()?,
        })
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_start_urls(mut self, urls: Vec<Url>) -> Self {
        self.start_urls = urls;
        self
    }
}

impl Spider for BookSpider {
    fn name(&self) -> String {
        "example_spider".to_string()
    }

    fn start_urls(&self) -> Vec<Url> {
        self.start_urls.clone()
    }

    fn max_depth(&self) -> usize {
        self.max_depth
    }

    fn parser(&self) -> &dyn Parser {
        &self.parser
    }
}
