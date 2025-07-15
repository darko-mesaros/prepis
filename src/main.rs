use clap::Parser;
use std::path::{Path, PathBuf};
use std::fs;
use thiserror::Error;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// AWS clients container
struct AwsClients {
    s3_client: aws_sdk_s3::Client,
    transcribe_client: aws_sdk_transcribe::Client,
}

/// Initialize AWS clients using the default credential chain
async fn create_aws_clients() -> Result<AwsClients, AppError> {
    println!("🔧 Initializing AWS clients...");
    
    // Load AWS configuration from environment with behavior version
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    
    // Create S3 and Transcribe clients
    let s3_client = aws_sdk_s3::Client::new(&config);
    let transcribe_client = aws_sdk_transcribe::Client::new(&config);
    
    // Test AWS credentials by making a simple call
    match s3_client.list_buckets().send().await {
        Ok(_) => {
            println!("✓ AWS credentials validated successfully");
        }
        Err(e) => {
            return Err(AppError::Aws(format!(
                "Failed to validate AWS credentials: {}. Please check your AWS configuration.",
                e
            )));
        }
    }
    
    Ok(AwsClients {
        s3_client,
        transcribe_client,
    })
}

/// Generate a unique S3 key based on filename and timestamp
fn generate_s3_key(file_path: &Path) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let filename = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    
    format!("transcribe-temp/{}-{}", timestamp, filename)
}

/// Upload a file to S3 and return the S3 URI
async fn upload_file_to_s3(
    s3_client: &aws_sdk_s3::Client,
    bucket: &str,
    file_path: &Path,
) -> Result<String, AppError> {
    let s3_key = generate_s3_key(file_path);
    
    println!("📤 Uploading file to S3: s3://{}/{}", bucket, s3_key);
    
    // Read the file content
    let file_content = tokio::fs::read(file_path).await?;
    let file_size = file_content.len();
    
    // Create the put object request
    let put_object_req = s3_client
        .put_object()
        .bucket(bucket)
        .key(&s3_key)
        .body(file_content.into());
    
    // Upload the file
    match put_object_req.send().await {
        Ok(_) => {
            println!("✓ File uploaded successfully ({:.2} MB)", 
                     file_size as f64 / (1024.0 * 1024.0));
            Ok(format!("s3://{}/{}", bucket, s3_key))
        }
        Err(e) => {
            Err(AppError::S3(format!("Failed to upload file to S3: {}", e)))
        }
    }
}

/// Delete a file from S3
async fn delete_file_from_s3(
    s3_client: &aws_sdk_s3::Client,
    bucket: &str,
    s3_key: &str,
) -> Result<(), AppError> {
    println!("🗑️  Cleaning up S3 file: s3://{}/{}", bucket, s3_key);
    
    match s3_client
        .delete_object()
        .bucket(bucket)
        .key(s3_key)
        .send()
        .await
    {
        Ok(_) => {
            println!("✓ S3 file deleted successfully");
            Ok(())
        }
        Err(e) => {
            // Log warning but don't fail the operation
            eprintln!("⚠️  Warning: Failed to delete S3 file: {}", e);
            Ok(())
        }
    }
}

/// Generate a unique transcription job name using timestamp and filename
fn generate_job_name(file_path: &Path) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let filename = file_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    
    format!("transcribe-job-{}-{}", timestamp, filename)
}

/// Start a transcription job with Amazon Transcribe
async fn start_transcription_job(
    transcribe_client: &aws_sdk_transcribe::Client,
    job_name: &str,
    s3_uri: &str,
) -> Result<(), AppError> {
    println!("🎙️  Starting transcription job: {}", job_name);
    
    // Create the media object with the S3 URI
    let media = aws_sdk_transcribe::types::Media::builder()
        .media_file_uri(s3_uri)
        .build();
    
    // Start the transcription job
    match transcribe_client
        .start_transcription_job()
        .transcription_job_name(job_name)
        .media(media)
        .language_code(aws_sdk_transcribe::types::LanguageCode::EnUs) // Default to English
        .send()
        .await
    {
        Ok(_) => {
            println!("✓ Transcription job started successfully");
            Ok(())
        }
        Err(e) => {
            Err(AppError::Transcribe(format!("Failed to start transcription job: {}", e)))
        }
    }
}

/// Transcription job status enum
#[derive(Debug)]
enum TranscriptionStatus {
    InProgress,
    Completed(String), // Contains result URI
    Failed(String),    // Contains failure reason
}

/// Poll transcription job status with exponential backoff
async fn poll_transcription_status(
    transcribe_client: &aws_sdk_transcribe::Client,
    job_name: &str,
) -> Result<TranscriptionStatus, AppError> {
    println!("⏳ Polling transcription job status...");
    
    let mut interval = 5; // Start with 5 seconds
    let max_interval = 30; // Maximum 30 seconds
    let max_attempts = 120; // Maximum attempts (10 minutes total)
    
    for attempt in 1..=max_attempts {
        println!("🔍 Checking status (attempt {}/{})", attempt, max_attempts);
        
        match transcribe_client
            .get_transcription_job()
            .transcription_job_name(job_name)
            .send()
            .await
        {
            Ok(response) => {
                if let Some(job) = response.transcription_job() {
                    match job.transcription_job_status() {
                        Some(aws_sdk_transcribe::types::TranscriptionJobStatus::InProgress) => {
                            println!("⏳ Job still in progress...");
                        }
                        Some(aws_sdk_transcribe::types::TranscriptionJobStatus::Completed) => {
                            if let Some(transcript) = job.transcript() {
                                if let Some(uri) = transcript.transcript_file_uri() {
                                    println!("✓ Transcription job completed successfully");
                                    return Ok(TranscriptionStatus::Completed(uri.to_string()));
                                }
                            }
                            return Err(AppError::Transcribe("Job completed but no transcript URI found".to_string()));
                        }
                        Some(aws_sdk_transcribe::types::TranscriptionJobStatus::Failed) => {
                            let failure_reason = job.failure_reason()
                                .unwrap_or("Unknown failure reason")
                                .to_string();
                            return Ok(TranscriptionStatus::Failed(failure_reason));
                        }
                        _ => {
                            return Err(AppError::Transcribe("Unknown job status".to_string()));
                        }
                    }
                } else {
                    return Err(AppError::Transcribe("Job not found".to_string()));
                }
            }
            Err(e) => {
                return Err(AppError::Transcribe(format!("Failed to get job status: {}", e)));
            }
        }
        
        // Wait before next attempt
        if attempt < max_attempts {
            println!("⏰ Waiting {} seconds before next check...", interval);
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
            
            // Increase interval with exponential backoff, but cap at max_interval
            interval = std::cmp::min(interval * 2, max_interval);
        }
    }
    
    Err(AppError::Transcribe("Transcription job timed out".to_string()))
}

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
    
    // Initialize AWS clients
    let aws_clients = create_aws_clients().await?;
    
    // Upload file to S3
    let s3_uri = upload_file_to_s3(&aws_clients.s3_client, &args.s3_bucket, &args.video_file).await?;
    println!("📍 S3 URI: {}", s3_uri);
    
    // Start transcription job
    let job_name = generate_job_name(&args.video_file);
    start_transcription_job(&aws_clients.transcribe_client, &job_name, &s3_uri).await?;
    
    // Poll for completion
    let transcription_status = poll_transcription_status(&aws_clients.transcribe_client, &job_name).await?;
    
    match transcription_status {
        TranscriptionStatus::Completed(result_uri) => {
            println!("🎉 Transcription completed! Result URI: {}", result_uri);
            // TODO: Retrieve and display results
        }
        TranscriptionStatus::Failed(reason) => {
            return Err(AppError::Transcribe(format!("Transcription failed: {}", reason)));
        }
        TranscriptionStatus::InProgress => {
            return Err(AppError::Transcribe("Unexpected: job still in progress after polling".to_string()));
        }
    }
    
    // TODO: Retrieve and display results
    // TODO: Clean up resources
    
    Ok(())
}
