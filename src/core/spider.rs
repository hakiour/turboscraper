use async_trait::async_trait;
use url::Url;

use crate::parser::Parser;

#[derive(Debug, Clone)]
pub enum Callback {
    Parse,
    Custom(String),
}

#[async_trait]
pub trait Spider {
    fn name(&self) -> String;
    fn start_urls(&self) -> Vec<Url>;
    fn max_depth(&self) -> usize {
        2
    }
    
    fn parser(&self) -> &dyn Parser;

    fn allowed_domains(&self) -> Option<Vec<String>> {
        None
    }
} 