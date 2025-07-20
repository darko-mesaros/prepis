//! # Prepis - Video Transcription CLI
//!
//! This is the main entry point for the Prepis application, which provides
//! command-line functionality for transcribing video and audio files using
//! Amazon Transcribe.
//!
//! The application handles:
//! - Command-line argument parsing
//! - Orchestration of the transcription workflow
//! - User feedback and progress reporting
//!
//! ## Workflow
//!
//! 1. Parse command-line arguments
//! 2. Validate input file
//! 3. Upload file to S3
//! 4. Start Amazon Transcribe job
//! 5. Poll for job completion
//! 6. Retrieve and display results
//! 7. Clean up temporary S3 files

mod aws;
mod error;
mod file;
mod models;
mod utils;

use clap::Parser;
use error::AppError;
use models::TranscriptionStatus;
use std::path::PathBuf;

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

    /// Output filename for the transcription
    #[arg(help = "S3 bucket name for temporary storage")]
    output_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    if let Err(e) = run_transcription(args).await {
        error::display_error(&e);
        std::process::exit(1);
    }

    Ok(())
}

async fn run_transcription(args: CliArgs) -> Result<(), error::AppError> {
    println!("Video Transcription CLI");
    println!("Video file: {:?}", args.video_file);
    println!("S3 bucket: {}", args.s3_bucket);
    if let Some(filename) = &args.output_file {
        println!("Output file: {}", filename.to_string_lossy());
    }

    // Validate the video file
    file::validate_video_file(&args.video_file)?;

    // Initialize AWS clients
    let aws_clients = aws::create_aws_clients().await?;

    // Upload file to S3
    let s3_uri =
        aws::upload_file_to_s3(&aws_clients.s3_client, &args.s3_bucket, &args.video_file).await?;
    println!("📍 S3 URI: {}", s3_uri);

    // Start transcription job
    let job_name = utils::generate_job_name(&args.video_file);
    aws::start_transcription_job(&aws_clients.transcribe_client, &job_name, &s3_uri).await?;

    // Poll for completion
    let transcription_status =
        aws::poll_transcription_status(&aws_clients.transcribe_client, &job_name).await?;

    match transcription_status {
        TranscriptionStatus::Completed(result_uri) => {
            println!("🎉 Transcription completed! Result URI: {}", result_uri);

            // Retrieve and display results
            let transcript_text = aws::get_transcription_result(&result_uri).await?;
            println!("\n📝 Transcription Results:");
            println!("─────────────────────────");
            println!("{}", transcript_text);
            println!("─────────────────────────");

            if let Some(filename) = &args.output_file {
                println!("💾 Saving transcription to: {}", filename.to_string_lossy());
                file::save_transcription(filename, &transcript_text)?;
            }
        }
        TranscriptionStatus::Failed(reason) => {
            return Err(AppError::Transcribe(format!(
                "Transcription failed: {}",
                reason
            )));
        }
    }

    // Clean up resources
    let s3_key = utils::generate_s3_key(&args.video_file);
    aws::delete_file_from_s3(&aws_clients.s3_client, &args.s3_bucket, &s3_key).await?;

    Ok(())
}
