//! # File Operations
//!
//! This module handles all file-related operations in the Prepis application.
//!
//! It provides functionality for:
//! - File validation and verification
//! - File format checking
//! - File size validation
//! - Reading and writing transcription files
//!
//! The module ensures that files meet the requirements for Amazon Transcribe
//! before they are processed.

pub mod validation;
pub mod writing;

pub use validation::validate_video_file;
pub use writing::save_transcription;
