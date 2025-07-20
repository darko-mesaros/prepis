# Design Document

## Overview

This design implements a progress bar for S3 file uploads using the `indicatif` crate. The solution will modify the existing `upload_file_to_s3` function to provide real-time upload progress feedback. Since the AWS SDK for Rust doesn't natively support upload progress callbacks, we'll implement a streaming approach using multipart uploads for larger files and a custom progress wrapper for smaller files.

## Architecture

The progress bar implementation will consist of three main components:

1. **Progress Bar Manager**: Handles the creation, updating, and cleanup of progress bars using `indicatif`
2. **Upload Strategy**: Determines whether to use simple upload or multipart upload based on file size
3. **Progress Tracking**: Wraps the upload process to provide progress updates

### Component Interaction Flow

```
File Upload Request
       ↓
File Size Check
       ↓
┌─────────────────┬─────────────────┐
│   < 100MB       │    >= 100MB     │
│ Simple Upload   │ Multipart Upload│
└─────────────────┴─────────────────┘
       ↓                    ↓
Progress Bar Creation   Progress Bar Creation
       ↓                    ↓
Upload with Progress    Upload Parts with Progress
       ↓                    ↓
Progress Updates        Aggregate Progress Updates
       ↓                    ↓
       └─────────────────────┘
                ↓
        Complete & Cleanup
```

## Components and Interfaces

### 1. Progress Bar Manager

```rust
pub struct UploadProgressBar {
    progress_bar: ProgressBar,
    start_time: Instant,
}

impl UploadProgressBar {
    pub fn new(file_size: u64, file_name: &str) -> Self
    pub fn update(&self, bytes_uploaded: u64)
    pub fn finish_with_message(&self, message: String)
    pub fn abandon(&self)
}
```

### 2. Upload Strategy Enum

```rust
pub enum UploadStrategy {
    Simple,
    Multipart { part_size: usize },
}

impl UploadStrategy {
    pub fn determine(file_size: u64) -> Self
}
```

### 3. Progress Tracking Wrapper

```rust
pub struct ProgressTracker {
    progress_bar: UploadProgressBar,
    bytes_uploaded: Arc<AtomicU64>,
}

impl ProgressTracker {
    pub fn new(file_size: u64, file_name: &str) -> Self
    pub fn create_progress_stream(&self, data: Vec<u8>) -> impl Stream<Item = Bytes>
    pub fn update_progress(&self, additional_bytes: u64)
}
```

### 4. Modified S3 Upload Function

The existing `upload_file_to_s3` function will be refactored to:

```rust
pub async fn upload_file_to_s3(
    s3_client: &aws_sdk_s3::Client,
    bucket: &str,
    file_path: &Path,
) -> Result<String, AppError>
```

Internal implementation will delegate to:
- `upload_file_simple_with_progress()` for files < 100MB
- `upload_file_multipart_with_progress()` for files >= 100MB

## Data Models

### Progress Bar Configuration

```rust
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
```

### Upload Metrics

```rust
pub struct UploadMetrics {
    pub total_bytes: u64,
    pub uploaded_bytes: u64,
    pub start_time: Instant,
    pub current_speed: f64, // bytes per second
}

impl UploadMetrics {
    pub fn calculate_eta(&self) -> Duration
    pub fn calculate_speed(&self) -> f64
    pub fn progress_percentage(&self) -> f64
}
```

## Error Handling

### Progress Bar Error Scenarios

1. **Terminal Compatibility**: If the terminal doesn't support progress bars, fall back to periodic text updates
2. **File Size Unknown**: Use indeterminate progress bar with spinner
3. **Upload Interruption**: Ensure progress bar is properly cleaned up
4. **Concurrent Access**: Use atomic operations for thread-safe progress updates

### Error Recovery Strategy

```rust
pub enum ProgressError {
    TerminalNotSupported,
    FileSizeUnknown,
    UploadInterrupted,
}

impl From<ProgressError> for AppError {
    fn from(err: ProgressError) -> Self {
        match err {
            ProgressError::TerminalNotSupported => {
                // Fall back to text-based progress
                AppError::Warning("Progress bar not supported, using text updates".to_string())
            },
            ProgressError::FileSizeUnknown => {
                // Use indeterminate progress
                AppError::Warning("File size unknown, showing indeterminate progress".to_string())
            },
            ProgressError::UploadInterrupted => {
                AppError::S3("Upload was interrupted".to_string())
            },
        }
    }
}
```

## Testing Strategy

Testing will be handled separately and is not part of this initial implementation scope.

## Implementation Dependencies

### New Cargo Dependencies

```toml
[dependencies]
indicatif = "0.17"
tokio-stream = "0.1"
futures-util = "0.3"
```

### AWS SDK Considerations

- The AWS SDK for Rust doesn't provide built-in progress callbacks
- We'll need to implement progress tracking by wrapping the upload data stream
- For multipart uploads, we'll track progress across multiple parts
- Memory usage will be optimized by streaming data rather than loading entire files

## Performance Considerations

1. **Memory Usage**: Stream file data in chunks rather than loading entire files into memory
2. **Progress Update Frequency**: Limit progress updates to avoid overwhelming the terminal (max 10 updates per second)
3. **Multipart Threshold**: Use 100MB threshold to balance performance and progress granularity
4. **Atomic Operations**: Use `AtomicU64` for thread-safe progress tracking without locks