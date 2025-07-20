//! # S3 Operations
//!
//! This module handles all Amazon S3 operations for the Prepis application.
//!
//! It provides functionality for:
//! - Uploading files to S3 buckets
//! - Generating unique S3 keys for uploaded files
//! - Cleaning up temporary files after processing
//! - Error handling for S3 operations
//!
//! The module ensures that files are properly stored and cleaned up during
//! the transcription process.

use crate::error::AppError;
use crate::utils::generate_s3_key;
use std::path::Path;

/// Upload a file to S3 and return the S3 URI
pub async fn upload_file_to_s3(
    s3_client: &aws_sdk_s3::Client,
    bucket: &str,
    file_path: &Path,
) -> Result<String, AppError> {
    let s3_key = generate_s3_key(file_path);

    println!("📤 Uploading file to S3: s3://{}/{}", bucket, s3_key);
    println!("📤 NOTE: This file will be deleted at the end");

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
            println!(
                "✅ File uploaded successfully ({:.2} MB)",
                file_size as f64 / (1024.0 * 1024.0)
            );
            Ok(format!("s3://{}/{}", bucket, s3_key))
        }
        Err(e) => Err(AppError::S3(format!("Failed to upload file to S3: {}", e))),
    }
}

/// Delete a file from S3
pub async fn delete_file_from_s3(
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
            println!("✅ S3 file deleted successfully");
            Ok(())
        }
        Err(e) => {
            // Log warning but don't fail the operation
            eprintln!(
                "⚠️  Warning: Failed to delete S3 file, please do so manually: {}",
                e
            );
            Ok(())
        }
    }
}
