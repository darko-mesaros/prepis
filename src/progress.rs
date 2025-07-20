//! # Progress Bar Module
//!
//! This module provides progress bar functionality for S3 file uploads.
//! It uses the indicatif crate to display real-time upload progress,
//! including upload speed, percentage completion, and estimated time remaining.
//!
//! The module supports both simple uploads for smaller files and multipart
//! uploads for larger files, with appropriate progress tracking for each.


use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Configuration for progress bar appearance and behavior
pub struct ProgressConfig {
    pub template: &'static str,
    pub progress_chars: &'static str,
    pub steady_tick: Duration,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            template: "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
            progress_chars: "█▉▊▋▌▍▎▏  ",
            steady_tick: Duration::from_millis(100),
        }
    }
}

/// Strategy for determining upload approach based on file size
#[derive(Debug, Clone)]
pub enum UploadStrategy {
    Simple,
    Multipart { part_size: usize },
}

impl UploadStrategy {
    /// Determine upload strategy based on file size
    /// Files >= 50MB use multipart upload
    pub fn determine(file_size: u64) -> Self {
        const MULTIPART_THRESHOLD: u64 = 50 * 1024 * 1024; // 50MB
        const PART_SIZE: usize = 8 * 1024 * 1024; // 8MB parts

        if file_size >= MULTIPART_THRESHOLD {
            Self::Multipart { part_size: PART_SIZE }
        } else {
            Self::Simple
        }
    }
}/// Progress bar wrapper for S3 upload operations
pub struct UploadProgressBar {
    progress_bar: ProgressBar,
    start_time: Instant,
    file_name: String,
}

impl UploadProgressBar {
    /// Create a new progress bar for file upload
    pub fn new(file_size: u64, file_name: &str) -> Self {
        let config = ProgressConfig::default();
        
        let progress_bar = ProgressBar::new(file_size);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(config.template)
                .expect("Invalid progress bar template")
                .progress_chars(config.progress_chars),
        );
        
        progress_bar.enable_steady_tick(config.steady_tick);
        
        // Set initial message
        progress_bar.set_message(format!("Uploading {}", file_name));
        
        Self {
            progress_bar,
            start_time: Instant::now(),
            file_name: file_name.to_string(),
        }
    }

    /// Create an indeterminate progress bar when file size is unknown
    pub fn new_indeterminate(file_name: &str) -> Self {
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] Uploading {msg}")
                .expect("Invalid spinner template"),
        );
        
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        progress_bar.set_message(file_name.to_string());
        
        Self {
            progress_bar,
            start_time: Instant::now(),
            file_name: file_name.to_string(),
        }
    }

    /// Update progress with the number of bytes uploaded
    pub fn update(&self, bytes_uploaded: u64) {
        self.progress_bar.set_position(bytes_uploaded);
    }

    /// Increment progress by additional bytes
    pub fn increment(&self, additional_bytes: u64) {
        self.progress_bar.inc(additional_bytes);
    }

    /// Finish the progress bar with default success message
    pub fn finish(&self) {
        let elapsed = self.start_time.elapsed();
        let message = format!(
            "✅ {} uploaded successfully in {:.1}s",
            self.file_name,
            elapsed.as_secs_f64()
        );
        self.progress_bar.finish_with_message(message);
    }

    /// Abandon the progress bar (for error cases)
    pub fn abandon(&self) {
        self.progress_bar.abandon_with_message(format!(
            "❌ Upload of {} was interrupted",
            self.file_name
        ));
    }

    /// Check if the terminal supports progress bars
    pub fn is_terminal_supported() -> bool {
        // Check if we're in a terminal that supports progress bars
        atty::is(atty::Stream::Stderr) || atty::is(atty::Stream::Stdout)
    }
}

///Thread-safe progress tracker for upload operations
pub struct ProgressTracker {
    progress_bar: UploadProgressBar,
    bytes_uploaded: Arc<AtomicU64>,
    total_bytes: u64,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(file_size: u64, file_name: &str) -> Self {
        let progress_bar = if UploadProgressBar::is_terminal_supported() {
            UploadProgressBar::new(file_size, file_name)
        } else {
            UploadProgressBar::new_indeterminate(file_name)
        };

        Self {
            progress_bar,
            bytes_uploaded: Arc::new(AtomicU64::new(0)),
            total_bytes: file_size,
        }
    }

    /// Create a progress tracker for unknown file size
    pub fn new_indeterminate(file_name: &str) -> Self {
        let progress_bar = UploadProgressBar::new_indeterminate(file_name);

        Self {
            progress_bar,
            bytes_uploaded: Arc::new(AtomicU64::new(0)),
            total_bytes: 0,
        }
    }

    /// Update progress with additional bytes uploaded
    pub fn update_progress(&self, additional_bytes: u64) {
        let new_total = self.bytes_uploaded.fetch_add(additional_bytes, Ordering::Relaxed) + additional_bytes;
        
        if self.total_bytes > 0 {
            self.progress_bar.update(new_total);
        } else {
            // For indeterminate progress, just increment
            self.progress_bar.increment(additional_bytes);
        }
    }

    /// Finish the progress tracker successfully
    pub fn finish(&self) {
        self.progress_bar.finish();
    }

    /// Abandon the progress tracker (for errors)
    pub fn abandon(&self) {
        self.progress_bar.abandon();
    }

}

