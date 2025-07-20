//! # Utility Functions
//!
//! This module provides general-purpose utility functions used throughout
//! the Prepis application.
//!
//! It includes:
//! - Functions for generating unique identifiers
//! - Time-based utilities
//! - Path and filename manipulation helpers
//!
//! These utilities are designed to be reusable and independent of specific
//! application logic.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a unique S3 key based on filename and timestamp
pub fn generate_s3_key(file_path: &Path) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap() // TODO: Handle unwrap
        .as_secs();

    let filename = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    format!("transcribe-temp/{}-{}", timestamp, filename)
}

/// Generate a unique transcription job name using timestamp and filename
pub fn generate_job_name(file_path: &Path) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap() // TODO: Handle unwrap
        .as_secs();

    let filename = file_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    format!("transcribe-job-{}-{}", timestamp, filename)
}
