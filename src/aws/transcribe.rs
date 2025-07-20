//! # Amazon Transcribe Operations
//!
//! This module handles all interactions with the Amazon Transcribe service.
//!
//! It provides functionality for:
//! - Starting transcription jobs
//! - Monitoring job status with exponential backoff
//! - Retrieving and parsing transcription results
//! - Error handling for transcription operations
//!
//! The module implements a robust polling mechanism to efficiently wait for
//! transcription jobs to complete while providing user feedback.

use crate::error::AppError;
use crate::models::TranscriptionStatus;

/// Start a transcription job with Amazon Transcribe
pub async fn start_transcription_job(
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
            println!("✅ Transcription job started successfully");
            Ok(())
        }
        Err(e) => Err(AppError::Transcribe(format!(
            "🛑 Error: Failed to start transcription job: {}",
            e
        ))),
    }
}

/// Poll transcription job status with exponential backoff
pub async fn poll_transcription_status(
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
                                    println!("✅ Transcription job completed successfully");
                                    return Ok(TranscriptionStatus::Completed(uri.to_string()));
                                }
                            }
                            return Err(AppError::Transcribe(
                                "Job completed but no transcript URI found".to_string(),
                            ));
                        }
                        Some(aws_sdk_transcribe::types::TranscriptionJobStatus::Failed) => {
                            let failure_reason = job
                                .failure_reason()
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
                return Err(AppError::Transcribe(format!(
                    "Failed to get job status: {}",
                    e
                )));
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

    Err(AppError::Transcribe(
        "Transcription job timed out".to_string(),
    ))
}

/// Retrieve and parse transcription results from the result URI
pub async fn get_transcription_result(result_uri: &str) -> Result<String, AppError> {
    println!("📥 Retrieving transcription results...");

    // Make HTTP request to get the transcription JSON
    let response = reqwest::get(result_uri).await.map_err(|e| {
        AppError::Transcribe(format!("Failed to fetch transcription results: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(AppError::Transcribe(format!(
            "Failed to fetch transcription results: HTTP {}",
            response.status()
        )));
    }

    let json_text = response.text().await.map_err(|e| {
        AppError::Transcribe(format!("Failed to read transcription response: {}", e))
    })?;

    // Parse the JSON to extract the transcript text
    let json_value: serde_json::Value = serde_json::from_str(&json_text)
        .map_err(|e| AppError::Transcribe(format!("Failed to parse transcription JSON: {}", e)))?;

    // Navigate the JSON structure to extract the transcript text
    let transcript_text = json_value
        .get("results")
        .and_then(|results| results.get("transcripts"))
        .and_then(|transcripts| transcripts.as_array())
        .and_then(|arr| arr.first())
        .and_then(|transcript| transcript.get("transcript"))
        .and_then(|text| text.as_str())
        .ok_or_else(|| AppError::Transcribe("No transcript text found in results".to_string()))?;
    // TODO: This 👆 can be cleaner as a Struct

    if transcript_text.trim().is_empty() {
        return Err(AppError::Transcribe(
            "Transcription result is empty".to_string(),
        ));
    }

    println!("✅ Transcription results retrieved successfully");
    Ok(transcript_text.to_string())
}
