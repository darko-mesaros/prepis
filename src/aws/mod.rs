//! # AWS Integration
//!
//! This module handles all AWS service interactions for the Prepis application.
//!
//! It provides:
//! - AWS client initialization and configuration
//! - S3 operations for file storage and retrieval
//! - Amazon Transcribe job management
//! - Result processing and parsing
//!
//! The module abstracts away the complexities of working with AWS services
//! and provides a clean interface for the main application.

pub mod client;
pub mod s3;
pub mod transcribe;

pub use client::create_aws_clients;
pub use s3::delete_file_from_s3;
pub use s3::upload_file_to_s3;
pub use transcribe::get_transcription_result;
pub use transcribe::poll_transcription_status;
pub use transcribe::start_transcription_job;
