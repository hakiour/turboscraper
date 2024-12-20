use crate::errors::ScraperResult;
use crate::scraper::Response;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

pub struct Storage {
    output_dir: PathBuf,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> ScraperResult<Self> {
        let output_dir = output_dir.as_ref().to_path_buf();
        fs::create_dir_all(&output_dir)?;
        Ok(Self { output_dir })
    }

    pub fn save_response(&self, response: &Response) -> ScraperResult<PathBuf> {
        let timestamp = response.timestamp.format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}", timestamp, sanitize_filename(&response.url));

        let json_path = self.output_dir.join(format!("{}.json", filename));
        let data = serde_json::json!({
            "metadata": {
                "url": response.url.to_string(),
                "status": response.status,
                "headers": response.headers,
                "timestamp": response.timestamp,
            },
            "content": {
                "html": response.body
            }
        });

        fs::write(json_path.clone(), serde_json::to_string_pretty(&data)?)?;

        Ok(json_path)
    }
}

fn sanitize_filename(url: &Url) -> String {
    let host = url.host_str().unwrap_or("unknown");
    let path = url.path().replace('/', "_");
    format!("{}{}", host, path)
        .replace([':', '?', '#', '&', '='], "_")
        .replace(".", "_")
}
