//! # Error Handling
//!
//! This module defines the error types and handling mechanisms used throughout
//! the Prepis application.
//!
//! It provides:
//! - A central `AppError` enum for all application errors
//! - Conversions from AWS SDK errors to application errors
//! - User-friendly error display functionality
//!
//! The error system is designed to provide clear, actionable feedback to users
//! when something goes wrong during the transcription process.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("File error: {0}")]
    File(String),

    #[error("AWS error: {0}")]
    Aws(String),

    #[error("S3 error: {0}")]
    S3(String),

    #[error("Transcribe error: {0}")]
    Transcribe(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Error conversion implementations
impl From<aws_sdk_s3::Error> for AppError {
    fn from(err: aws_sdk_s3::Error) -> Self {
        AppError::S3(err.to_string())
    }
}

impl From<aws_sdk_transcribe::Error> for AppError {
    fn from(err: aws_sdk_transcribe::Error) -> Self {
        AppError::Transcribe(err.to_string())
    }
}

/// Display error messages in a user-friendly format
pub fn display_error(error: &AppError) {
    eprintln!("🛑 Error: {}", error);

    // Display additional context for specific error types
    match error {
        AppError::File(_) => {
            eprintln!("Please verify the file path and permissions.");
        }
        AppError::Aws(_) => {
            eprintln!("Please check your AWS credentials and configuration.");
        }
        AppError::S3(_) => {
            eprintln!("Please verify the S3 bucket exists and you have access to it.");
        }
        AppError::Transcribe(_) => {
            eprintln!("Please check the Amazon Transcribe service status and your permissions.");
        }
        AppError::Io(_) => {
            eprintln!("Please check file permissions and disk space.");
        }
    }
}
