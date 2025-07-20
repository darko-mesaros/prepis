//! # File Writing
//!
//! This module provides functionality for writing output files to disk
//!
//! It handles:
//! - Saving transcription results to disk.

use crate::error::AppError;
use std::fs;
use std::path::Path;

/// Save transcription to disk
pub fn save_transcription(path: impl AsRef<Path>, content: &str) -> Result<(), AppError> {
    fs::write(path, content)?;
    Ok(())
}
