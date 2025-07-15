# Design Document

## Overview

The video transcription CLI application is a Rust-based command-line tool that leverages Amazon Transcribe to convert video files into text transcriptions. The application follows a simple workflow: upload video file to S3, start transcription job, poll for completion, retrieve results, and clean up resources.

The application is designed for simplicity and uses the AWS SDK for Rust to interact with AWS services. It relies on the default AWS credential chain for authentication and requires users to provide both a video file path and an S3 bucket name as command-line arguments.

## Architecture

The application follows a linear, synchronous workflow with the following high-level components:

```
CLI Input → File Validation → S3 Upload → Transcribe Job → Poll Status → Retrieve Results → Cleanup → Output
```

### Core Components:

1. **CLI Parser**: Handles command-line argument parsing and validation
2. **AWS Client Manager**: Manages AWS SDK clients for S3 and Transcribe services
3. **File Handler**: Validates and processes input video files
4. **S3 Manager**: Handles file upload and cleanup operations
5. **Transcription Manager**: Manages transcription job lifecycle
6. **Output Handler**: Formats and displays transcription results

## Components and Interfaces

### CLI Parser
- **Purpose**: Parse and validate command-line arguments
- **Dependencies**: `clap` crate for argument parsing
- **Interface**:
  ```rust
  struct CliArgs {
      video_file: PathBuf,
      s3_bucket: String,
  }
  
  fn parse_args() -> Result<CliArgs, CliError>
  ```

### AWS Client Manager
- **Purpose**: Initialize and manage AWS service clients
- **Dependencies**: `aws-config`, `aws-sdk-s3`, `aws-sdk-transcribe`
- **Interface**:
  ```rust
  struct AwsClients {
      s3_client: aws_sdk_s3::Client,
      transcribe_client: aws_sdk_transcribe::Client,
  }
  
  async fn create_clients() -> Result<AwsClients, AwsError>
  ```

### File Handler
- **Purpose**: Validate video file existence and format
- **Interface**:
  ```rust
  fn validate_video_file(path: &Path) -> Result<(), FileError>
  fn get_file_extension(path: &Path) -> Option<String>
  ```

### S3 Manager
- **Purpose**: Handle S3 operations for file upload and cleanup
- **Interface**:
  ```rust
  async fn upload_file(
      client: &aws_sdk_s3::Client,
      bucket: &str,
      file_path: &Path
  ) -> Result<String, S3Error>
  
  async fn delete_file(
      client: &aws_sdk_s3::Client,
      bucket: &str,
      key: &str
  ) -> Result<(), S3Error>
  ```

### Transcription Manager
- **Purpose**: Manage Amazon Transcribe job lifecycle
- **Interface**:
  ```rust
  async fn start_transcription_job(
      client: &aws_sdk_transcribe::Client,
      job_name: &str,
      s3_uri: &str
  ) -> Result<(), TranscribeError>
  
  async fn poll_transcription_status(
      client: &aws_sdk_transcribe::Client,
      job_name: &str
  ) -> Result<TranscriptionStatus, TranscribeError>
  
  async fn get_transcription_result(
      client: &aws_sdk_transcribe::Client,
      job_name: &str
  ) -> Result<String, TranscribeError>
  ```

### Output Handler
- **Purpose**: Format and display transcription results
- **Interface**:
  ```rust
  fn display_transcription(text: &str)
  fn display_error(error: &dyn std::error::Error)
  ```

## Data Models

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("CLI argument error: {0}")]
    Cli(String),
    
    #[error("File error: {0}")]
    File(String),
    
    #[error("AWS error: {0}")]
    Aws(String),
    
    #[error("S3 error: {0}")]
    S3(String),
    
    #[error("Transcribe error: {0}")]
    Transcribe(String),
}
```

### Configuration
```rust
struct AppConfig {
    video_file: PathBuf,
    s3_bucket: String,
    job_name: String,
    s3_key: String,
}
```

### Transcription Status
```rust
enum TranscriptionStatus {
    InProgress,
    Completed(String), // Contains result URI
    Failed(String),    // Contains failure reason
}
```

## Error Handling

The application implements comprehensive error handling using Rust's `Result` type and the `thiserror` crate for structured error definitions. Error handling strategy:

1. **Input Validation**: Validate all inputs early and provide clear error messages
2. **AWS Service Errors**: Catch and translate AWS SDK errors into user-friendly messages
3. **Resource Cleanup**: Ensure S3 cleanup occurs even when errors happen using RAII patterns
4. **Graceful Degradation**: Continue with core functionality when non-critical operations fail

### Error Recovery:
- **File Upload Failures**: Retry once with exponential backoff
- **Transcription Job Failures**: Display detailed error message from AWS
- **Cleanup Failures**: Log warning but don't fail the main operation

## Testing Strategy

### Manual Testing
- Test with sample video files in various formats (MP4, MOV, AVI)
- Verify error handling with invalid file paths and formats
- Test with different S3 bucket configurations
- Validate transcription output quality and formatting

## Implementation Notes

### AWS SDK Usage
- Use the latest stable versions of AWS SDK crates
- Leverage the default credential chain for authentication
- Implement proper async/await patterns throughout

### Performance Considerations
- File uploads are synchronous but use streaming for large files
- Polling interval starts at 5 seconds and increases to 30 seconds maximum
- Memory usage is minimized by streaming file operations

### Security Considerations
- No credentials are stored or logged
- S3 objects are cleaned up after transcription
- File paths are validated to prevent directory traversal

### Supported Video Formats
Based on Amazon Transcribe documentation, supported formats include:
- MP4, MOV, AVI, FLV, MP3, WAV, FLAC
- Maximum file size: 2GB
- Maximum duration: 4 hours