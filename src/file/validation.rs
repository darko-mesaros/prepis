//! # File Validation
//!
//! This module provides functionality for validating input files before
//! they are processed for transcription.
//!
//! It ensures that:
//! - Files exist and are accessible
//! - File formats are supported by Amazon Transcribe
//! - File sizes are within acceptable limits
//! - Files are not empty or corrupted

use crate::error::AppError;
use std::fs;
use std::path::Path;

/// Supported video file extensions based on Amazon Transcribe documentation
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "flv", "mp3", "wav", "flac", "m4a", "webm", "mkv",
];

/// Maximum file size supported by Amazon Transcribe (2GB)
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2GB in bytes - TODO: Make this customizable

/// Validate that the video file exists, is readable, has a supported format, and is within size limits
pub fn validate_video_file(path: &Path) -> Result<(), AppError> {
    // Check if file exists
    if !path.exists() {
        return Err(AppError::File(format!(
            "File does not exist: {}",
            path.display()
        )));
    }

    // Check if it's a file (not a directory)
    if !path.is_file() {
        return Err(AppError::File(format!(
            "Path is not a file: {}",
            path.display()
        )));
    }

    // Validate file extension
    let extension = get_file_extension(path)
        .ok_or_else(|| AppError::File("File has no extension".to_string()))?;

    if !SUPPORTED_EXTENSIONS.contains(&extension.to_lowercase().as_str()) {
        return Err(AppError::File(format!(
            "Unsupported file format: {}. Supported formats: {}",
            extension,
            SUPPORTED_EXTENSIONS.join(", ")
        )));
    }

    // Check file size
    let metadata = fs::metadata(path)?;
    let file_size = metadata.len();

    if file_size > MAX_FILE_SIZE {
        return Err(AppError::File(format!(
            "File size ({:.2} MB) exceeds maximum limit of {} GB", // TODO: Make this flexibe MB/GB
            file_size as f64 / (1024.0 * 1024.0),
            MAX_FILE_SIZE
        )));
    }

    if file_size == 0 {
        return Err(AppError::File("File is empty".to_string()));
    }

    println!(
        "✅ File validation passed: {} ({:.2} MB)",
        path.display(),
        file_size as f64 / (1024.0 * 1024.0)
    );

    Ok(())
}

/// Extract file extension from path
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_string())
}
