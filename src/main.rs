use clap::Parser;
use std::path::{Path, PathBuf};
use std::fs;
use thiserror::Error;

#[derive(Debug, Error)]
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

// Additional AWS error conversions can be added as needed

/// Supported video file extensions based on Amazon Transcribe documentation
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "flv", "mp3", "wav", "flac", "m4a", "webm", "mkv"
];

/// Maximum file size supported by Amazon Transcribe (2GB)
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2GB in bytes

/// Validate that the video file exists, is readable, has a supported format, and is within size limits
fn validate_video_file(path: &Path) -> Result<(), AppError> {
    // Check if file exists
    if !path.exists() {
        return Err(AppError::File(format!("File does not exist: {}", path.display())));
    }
    
    // Check if it's a file (not a directory)
    if !path.is_file() {
        return Err(AppError::File(format!("Path is not a file: {}", path.display())));
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
            "File size ({:.2} MB) exceeds maximum limit of 2GB",
            file_size as f64 / (1024.0 * 1024.0)
        )));
    }
    
    if file_size == 0 {
        return Err(AppError::File("File is empty".to_string()));
    }
    
    println!("✓ File validation passed: {} ({:.2} MB)", 
             path.display(), 
             file_size as f64 / (1024.0 * 1024.0));
    
    Ok(())
}

/// Extract file extension from path
fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_string())
}

#[derive(Parser)]
#[command(name = "prepis")]
#[command(about = "A CLI tool to transcribe video files using Amazon Transcribe")]
#[command(version = "0.1.0")]
struct CliArgs {
    /// Path to the video file to transcribe
    #[arg(help = "Path to the video file")]
    video_file: PathBuf,
    
    /// S3 bucket name to use for temporary file storage
    #[arg(help = "S3 bucket name for temporary storage")]
    s3_bucket: String,
}

/// Display error messages in a user-friendly format
fn display_error(error: &AppError) {
    eprintln!("Error: {}", error);
    
    // Display additional context for specific error types
    match error {
        AppError::Cli(_) => {
            eprintln!("Please check your command line arguments.");
            eprintln!("Use --help for usage information.");
        }
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();
    
    if let Err(e) = run_transcription(args).await {
        display_error(&e);
        std::process::exit(1);
    }
    
    Ok(())
}

async fn run_transcription(args: CliArgs) -> Result<(), AppError> {
    println!("Video Transcription CLI");
    println!("Video file: {:?}", args.video_file);
    println!("S3 bucket: {}", args.s3_bucket);
    
    // Validate the video file
    validate_video_file(&args.video_file)?;
    
    // TODO: Initialize AWS clients
    // TODO: Upload file to S3
    // TODO: Start transcription job
    // TODO: Poll for completion
    // TODO: Retrieve and display results
    // TODO: Clean up resources
    
    Ok(())
}
