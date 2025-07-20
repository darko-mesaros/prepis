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
use crate::progress::{ProgressTracker, UploadStrategy};
use crate::utils::generate_s3_key;
use aws_sdk_s3::primitives::ByteStream;

use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// Upload a file to S3 and return the S3 URI
pub async fn upload_file_to_s3(
    s3_client: &aws_sdk_s3::Client,
    bucket: &str,
    file_path: &Path,
) -> Result<String, AppError> {
    let s3_key = generate_s3_key(file_path);
    
    // Get file metadata
    let metadata = tokio::fs::metadata(file_path).await?;
    let file_size = metadata.len();
    
    println!("📤 Uploading file to S3: s3://{}/{}", bucket, s3_key);
    println!("📤 NOTE: This file will be deleted at the end");
    
    // Determine upload strategy based on file size
    let strategy = UploadStrategy::determine(file_size);
    
    match strategy {
        UploadStrategy::Simple => {
            upload_file_simple_with_progress(s3_client, bucket, &s3_key, file_path, file_size).await
        }
        UploadStrategy::Multipart { part_size } => {
            upload_file_multipart_with_progress(s3_client, bucket, &s3_key, file_path, file_size, part_size).await
        }
    }
}

/// Upload a file using simple upload with progress tracking
async fn upload_file_simple_with_progress(
    s3_client: &aws_sdk_s3::Client,
    bucket: &str,
    s3_key: &str,
    file_path: &Path,
    file_size: u64,
) -> Result<String, AppError> {
    let file_name = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    // Create progress tracker with fallback support
    let progress_tracker = if file_size == 0 {
        ProgressTracker::new_indeterminate(file_name)
    } else {
        ProgressTracker::new(file_size, file_name)
    };
    
    // Read file in chunks to provide progress updates
    let mut file = File::open(file_path).await?;
    let mut buffer = Vec::with_capacity(file_size as usize);
    
    const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks
    let mut chunk_buffer = vec![0u8; CHUNK_SIZE];
    
    loop {
        let bytes_read = file.read(&mut chunk_buffer).await?;
        if bytes_read == 0 {
            break;
        }
        
        buffer.extend_from_slice(&chunk_buffer[..bytes_read]);
        progress_tracker.update_progress(bytes_read as u64);
        
        // Small delay to make progress visible for small files
        if file_size < 1024 * 1024 { // < 1MB
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }
    
    // Create the put object request
    let put_object_req = s3_client
        .put_object()
        .bucket(bucket)
        .key(s3_key)
        .body(ByteStream::from(buffer));
    
    // Upload the file
    match put_object_req.send().await {
        Ok(_) => {
            progress_tracker.finish();
            Ok(format!("s3://{}/{}", bucket, s3_key))
        }
        Err(e) => {
            progress_tracker.abandon();
            Err(AppError::S3(format!("Failed to upload file to S3: {}", e)))
        }
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
/// Upload a file using multipart upload with progress tracking
async fn upload_file_multipart_with_progress(
    s3_client: &aws_sdk_s3::Client,
    bucket: &str,
    s3_key: &str,
    file_path: &Path,
    file_size: u64,
    part_size: usize,
) -> Result<String, AppError> {
    let file_name = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    // Create progress tracker
    let progress_tracker = ProgressTracker::new(file_size, file_name);
    
    // Initialize multipart upload
    let create_multipart_upload_res = s3_client
        .create_multipart_upload()
        .bucket(bucket)
        .key(s3_key)
        .send()
        .await
        .map_err(|e| AppError::S3(format!("Failed to create multipart upload: {}", e)))?;
    
    let upload_id = create_multipart_upload_res
        .upload_id()
        .ok_or_else(|| AppError::S3("No upload ID returned".to_string()))?;
    
    // Open file for reading
    let mut file = File::open(file_path).await?;
    let mut part_number = 1i32; // Ensure correct type for AWS API
    let mut completed_parts = Vec::new();
    let mut buffer = vec![0u8; part_size];
    
    loop {
        // Read exactly part_size bytes, or whatever remains
        let mut total_read = 0;
        while total_read < part_size {
            let bytes_read = file.read(&mut buffer[total_read..]).await?;
            if bytes_read == 0 {
                break; // End of file
            }
            total_read += bytes_read;
        }
        
        if total_read == 0 {
            break; // No more data to read
        }
        
        // Upload this part
        let part_data = buffer[..total_read].to_vec();
        let upload_part_res = s3_client
            .upload_part()
            .bucket(bucket)
            .key(s3_key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(ByteStream::from(part_data))
            .send()
            .await
            .map_err(|e| {
                // If part upload fails, abort the multipart upload
                std::mem::drop(tokio::spawn({
                    let s3_client = s3_client.clone();
                    let bucket = bucket.to_string();
                    let s3_key = s3_key.to_string();
                    let upload_id = upload_id.to_string();
                    async move {
                        if let Err(e) = s3_client
                            .abort_multipart_upload()
                            .bucket(bucket)
                            .key(s3_key)
                            .upload_id(upload_id)
                            .send()
                            .await
                        {
                            eprintln!("⚠️  Warning: Failed to abort multipart upload: {}", e);
                        }
                    }
                }));
                AppError::S3(format!("Failed to upload part {}: {}", part_number, e))
            })?;
        
        // Store completed part info
        let etag = upload_part_res.e_tag()
            .ok_or_else(|| AppError::S3(format!("No ETag returned for part {}", part_number)))?;
        
        completed_parts.push(
            aws_sdk_s3::types::CompletedPart::builder()
                .part_number(part_number)
                .e_tag(etag)
                .build(),
        );
        
        // Update progress
        progress_tracker.update_progress(total_read as u64);
        part_number += 1;
    }
    
    // Check if we have any parts
    if completed_parts.is_empty() {
        progress_tracker.abandon();
        return Err(AppError::S3("No parts were successfully uploaded".to_string()));
    }
    
    // Complete the multipart upload
    let completed_multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();
    match s3_client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(s3_key)
        .upload_id(upload_id)
        .multipart_upload(completed_multipart_upload)
        .send()
        .await
    {
        Ok(_) => {
            progress_tracker.finish();
            Ok(format!("s3://{}/{}", bucket, s3_key))
        }
        Err(e) => {
            progress_tracker.abandon();
            
            // Attempt to abort the multipart upload
            if let Err(abort_err) = s3_client
                .abort_multipart_upload()
                .bucket(bucket)
                .key(s3_key)
                .upload_id(upload_id)
                .send()
                .await
            {
                eprintln!("⚠️  Warning: Failed to abort multipart upload: {}", abort_err);
            }
            
            Err(AppError::S3(format!("Failed to complete multipart upload: {}", e)))
        }
    }
}
