//! # Data Models
//!
//! This module contains the core data structures and enums used throughout
//! the Prepis application.
//!
//! These models represent the domain objects of the transcription process
//! and help maintain a clear separation between data and behavior.

/// Transcription job status enum
#[derive(Debug)]
pub enum TranscriptionStatus {
    Completed(String), // Contains result URI
    Failed(String),    // Contains failure reason
}
