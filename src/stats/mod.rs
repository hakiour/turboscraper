use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct ScrapingStats {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub retry_count: usize,
    pub bytes_downloaded: usize,
    pub status_codes: HashMap<u16, usize>,
    pub retry_reasons: HashMap<String, usize>,
    pub average_response_time: f64, // in milliseconds
}

#[derive(Debug, Clone)]
pub struct StatsTracker {
    stats: Arc<RwLock<ScrapingStats>>,
}

impl StatsTracker {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(ScrapingStats {
                start_time: Utc::now(),
                end_time: None,
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                retry_count: 0,
                bytes_downloaded: 0,
                status_codes: HashMap::new(),
                retry_reasons: HashMap::new(),
                average_response_time: 0.0,
            })),
        }
    }

    pub fn record_request(&self, status: u16, size: usize, duration: Duration) {
        let mut stats = self.stats.write();
        stats.total_requests += 1;

        if status < 400 {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        *stats.status_codes.entry(status).or_insert(0) += 1;
        stats.bytes_downloaded += size;

        // Update average response time
        let current_total = stats.average_response_time * (stats.total_requests - 1) as f64;
        let new_duration = duration.num_milliseconds() as f64;
        stats.average_response_time = (current_total + new_duration) / stats.total_requests as f64;
    }

    pub fn record_retry(&self, category: String) {
        let mut stats = self.stats.write();
        stats.retry_count += 1;
        *stats.retry_reasons.entry(category).or_insert(0) += 1;
    }

    pub fn finish(&self) {
        self.stats.write().end_time = Some(Utc::now());
    }

    pub fn get_stats(&self) -> ScrapingStats {
        self.stats.read().clone()
    }

    pub fn print_summary(&self) {
        let stats = self.stats.read();
        let duration = stats
            .end_time
            .unwrap_or_else(Utc::now)
            .signed_duration_since(stats.start_time);

        println!("\nScraping Statistics:");
        println!("===================");
        println!("Duration: {} seconds", duration.num_seconds());
        println!("Total Requests: {}", stats.total_requests);
        println!("Successful Requests: {}", stats.successful_requests);
        println!("Failed Requests: {}", stats.failed_requests);
        println!("Retry Count: {}", stats.retry_count);
        println!(
            "Data Downloaded: {:.2} MB",
            stats.bytes_downloaded as f64 / 1_000_000.0
        );
        println!(
            "Average Response Time: {:.2}ms",
            stats.average_response_time
        );

        println!("\nStatus Codes:");
        for (code, count) in &stats.status_codes {
            println!("  {}: {}", code, count);
        }

        if !stats.retry_reasons.is_empty() {
            println!("\nRetry Reasons:");
            for (reason, count) in &stats.retry_reasons {
                println!("  {}: {}", reason, count);
            }
        }
    }
}

impl Default for StatsTracker {
    fn default() -> Self {
        Self::new()
    }
}
