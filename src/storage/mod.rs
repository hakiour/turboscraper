use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;
use uuid::Uuid;

use crate::ScraperResult;

#[derive(Debug, Clone)]
pub struct StorageData {
    pub metadata: Metadata,
    pub content: Content,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub url: Url,
    pub timestamp: DateTime<Utc>,
    pub content_type: ContentType,
    pub extra: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum Content {
    Html(String),
    Json(Value),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Html,
    Json,
    Text,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Html => write!(f, "html"),
            ContentType::Json => write!(f, "json"),
            ContentType::Text => write!(f, "text"),
        }
    }
}

pub struct Storage {
    output_dir: PathBuf,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> ScraperResult<Self> {
        let output_dir = output_dir.as_ref().to_path_buf();
        fs::create_dir_all(&output_dir)?;
        Ok(Self { output_dir })
    }

    pub fn save(&self, data: StorageData) -> ScraperResult<PathBuf> {
        let timestamp = data.metadata.timestamp.format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}", timestamp, sanitize_filename(&data.metadata.url));
        let path = self.output_dir.join(format!("{}.json", filename));

        let json_data = serde_json::json!({
            "metadata": {
                "url": data.metadata.url.to_string(),
                "timestamp": data.metadata.timestamp,
                "content_type": data.metadata.content_type.to_string(),
                "extra": data.metadata.extra,
            },
            "content": match data.content {
                Content::Html(html) => html,
                Content::Json(json) => json.to_string(),
                Content::Text(text) => text,
            }
        });

        fs::write(&path, serde_json::to_string_pretty(&json_data)?)?;
        Ok(path)
    }
}

fn sanitize_filename(url: &Url) -> String {
    let host = url.host_str().unwrap_or("unknown");
    format!("{}_{}", host, Uuid::now_v7())
        .replace([':', '?', '#', '&', '='], "_")
        .replace(".", "_")
}
