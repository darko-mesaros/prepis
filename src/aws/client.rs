//! # AWS Client Management
//!
//! This module handles the initialization and management of AWS service clients
//! used throughout the Prepis application.
//!
//! It provides:
//! - AWS client initialization with proper configuration
//! - Credential validation
//! - Client structure for easy access to different AWS services
//!
//! The module ensures that AWS clients are properly configured before
//! any AWS operations are performed.

use crate::error::AppError;

/// AWS clients container
pub struct AwsClients {
    pub s3_client: aws_sdk_s3::Client,
    pub transcribe_client: aws_sdk_transcribe::Client,
}

/// Initialize AWS clients using the default credential chain
pub async fn create_aws_clients() -> Result<AwsClients, AppError> {
    println!("🔧 Initializing AWS clients...");

    // Load AWS configuration from environment with behavior version
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

    // Create S3 and Transcribe clients
    let s3_client = aws_sdk_s3::Client::new(&config);
    let transcribe_client = aws_sdk_transcribe::Client::new(&config);

    // Test AWS credentials by making a simple call
    match s3_client.list_buckets().send().await {
        Ok(_) => {
            println!("✅ AWS credentials validated successfully");
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
