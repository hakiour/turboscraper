use chrono::Duration;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug, Default)]
pub struct ScrapingStats {
    pub duration: Duration,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub retry_count: u64,
    pub data_downloaded: f64,
    pub total_response_time: u64,
    pub status_codes: HashMap<u16, u64>,
    pub retry_reasons: HashMap<String, u64>,
    pub storage_errors: u64,
}

pub struct StatsTracker {
    start_time: Instant,
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    retry_count: AtomicU64,
    data_downloaded: AtomicU64,
    total_response_time: AtomicU64,
    status_codes: parking_lot::RwLock<HashMap<u16, u64>>,
    retry_reasons: parking_lot::RwLock<HashMap<String, u64>>,
    storage_errors: AtomicU64,
    parsing_errors: AtomicU64,
    unhandled_errors: AtomicU64,
}

impl StatsTracker {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            retry_count: AtomicU64::new(0),
            data_downloaded: AtomicU64::new(0),
            total_response_time: AtomicU64::new(0),
            status_codes: parking_lot::RwLock::new(HashMap::new()),
            retry_reasons: parking_lot::RwLock::new(HashMap::new()),
            storage_errors: AtomicU64::new(0),
            parsing_errors: AtomicU64::new(0),
            unhandled_errors: AtomicU64::new(0),
        }
    }

    pub fn increment_storage_errors(&self) {
        self.storage_errors.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn increment_parsing_errors(&self) {
        self.parsing_errors.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn increment_unhandled_errors(&self) {
        self.unhandled_errors.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_request(&self, status: u16, size: usize, duration: Duration) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if status < 400 {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        let mut status_codes = self.status_codes.write();
        *status_codes.entry(status).or_insert(0) += 1;

        self.data_downloaded
            .fetch_add(size as u64, Ordering::Relaxed);
        self.total_response_time
            .fetch_add(duration.num_milliseconds() as u64, Ordering::Relaxed);
    }

    pub fn record_retry(&self, category: String) {
        self.retry_count.fetch_add(1, Ordering::Relaxed);
        let mut retry_reasons = self.retry_reasons.write();
        *retry_reasons.entry(category).or_insert(0) += 1;
    }

    pub fn get_stats(&self) -> ScrapingStats {
        ScrapingStats {
            duration: chrono::Duration::from_std(self.start_time.elapsed()).unwrap(),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            retry_count: self.retry_count.load(Ordering::Relaxed),
            data_downloaded: (self.data_downloaded.load(Ordering::Relaxed) as f64)
                / (1024.0 * 1024.0),
            total_response_time: self.total_response_time.load(Ordering::Relaxed),
            status_codes: self.status_codes.read().clone(),
            retry_reasons: self.retry_reasons.read().clone(),
            storage_errors: self.storage_errors.load(Ordering::Relaxed),
        }
    }

    pub fn print_summary(&self) {
        let stats = self.get_stats();
        println!("\nScraping Statistics:");
        println!("===================");
        println!("Duration: {} seconds", stats.duration.num_seconds());
        println!("Total Requests: {}", stats.total_requests);
        println!("Successful Requests: {}", stats.successful_requests);
        println!("Failed Requests: {}", stats.failed_requests);
        println!("Storage Errors: {}", stats.storage_errors);
        println!("Retry Count: {}", stats.retry_count);
        println!("Data Downloaded: {:.2} MB", stats.data_downloaded);

        if stats.total_requests > 0 {
            let avg_response_time = stats.total_response_time as f64 / stats.total_requests as f64;
            println!("Average Response Time: {:.2}ms", avg_response_time);
        }

        if !stats.status_codes.is_empty() {
            println!("\nStatus Codes:");
            for (code, count) in stats.status_codes.iter() {
                println!("  {}: {}", code, count);
            }
        }

        if !stats.retry_reasons.is_empty() {
            println!("\nRetry Reasons:");
            for (reason, count) in stats.retry_reasons.iter() {
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
