# Prepis - Video Transcription CLI

Sometimes you just need to have a transcription of video and audio files, very very fast. Well, now you don't need to leave the command line for this.

![prepis in action](/img/prepis.gif)

## Features

- 🎥 **Multiple Format Support** - MP4, MOV, AVI, FLV, MP3, WAV, FLAC, M4A, WebM, MKV
- ☁️ **AWS Integration** - Uses Amazon Transcribe for high-quality transcription
- 📊 **Progress Tracking** - Real-time status updates with visual indicators
- 🛡️ **Error Handling** - Comprehensive error messages with helpful guidance
- 🧹 **Auto Cleanup** - Automatically removes temporary S3 files
- 😎 **Emojis** - I like to have my CLI output feature a lot of emojis, you've been warned

## Prerequisites

- **Rust** (latest stable version)
- **AWS Account** with appropriate permissions
- **AWS CLI configured** or environment variables set
- **S3 Bucket** for temporary file storage

### Required AWS Permissions

Your AWS credentials need the following permissions:
- `s3:PutObject` - Upload files to S3
- `s3:DeleteObject` - Clean up temporary files
- `s3:ListBuckets` - Validate credentials
- `transcribe:StartTranscriptionJob` - Start transcription jobs
- `transcribe:GetTranscriptionJob` - Check job status

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd prepis
```

2. Build the project:
```bash
cargo build --release
```

3. (Optional) Install globally:
```bash
cargo install --path .
```

## Usage

### Basic Usage

```bash
cargo run -- <VIDEO_FILE> <S3_BUCKET>
```

### Examples

```bash
# Transcribe a local MP4 file
cargo run -- my-video.mp4 my-transcription-bucket

# Transcribe an audio file
cargo run -- podcast.mp3 my-transcription-bucket

# Using the installed binary
prepis presentation.mov company-transcripts
```

### Help

```bash
cargo run -- --help
```

## Configuration

### AWS Credentials

The application uses the standard AWS credential chain:

1. **Environment Variables**:
   ```bash
   export AWS_ACCESS_KEY_ID=your-access-key
   export AWS_SECRET_ACCESS_KEY=your-secret-key
   export AWS_DEFAULT_REGION=us-east-1
   ```

2. **AWS CLI Configuration**:
   ```bash
   aws configure
   ```

3. **IAM Roles** (when running on EC2)

4. **AWS Profiles**:
   ```bash
   export AWS_PROFILE=your-profile
   ```

### S3 Bucket Setup

Create an S3 bucket for temporary file storage, or you can use your existing one - just be mindful of the implications:

```bash
aws s3 mb s3://your-transcription-bucket
```

**Note**: Files are automatically deleted after transcription completes.

## How It Works

1. **Validation** - Checks file format, size (max 2GB), and existence
2. **Upload** - Securely uploads file to your S3 bucket with unique naming
3. **Transcription** - Starts Amazon Transcribe job with English language detection
4. **Polling** - Monitors job status with exponential backoff (5s → 30s intervals)
5. **Retrieval** - Downloads and parses transcription results
6. **Display** - Shows formatted transcription text
7. **Cleanup** - Removes temporary S3 files

## Supported File Formats

| Video | Audio | Max Size | Max Duration |
|-------|-------|----------|--------------|
| MP4, MOV, AVI, FLV, WebM, MKV | MP3, WAV, FLAC, M4A | 2GB | 4 hours |

## Example Output

```
Video Transcription CLI
Video file: "presentation.mp4"
S3 bucket: my-bucket

✓ File validation passed: presentation.mp4 (15.23 MB)
🔧 Initializing AWS clients...
✓ AWS credentials validated successfully
📤 Uploading file to S3: s3://my-bucket/transcribe-temp/1642123456-presentation.mp4
✓ File uploaded successfully (15.23 MB)
📍 S3 URI: s3://my-bucket/transcribe-temp/1642123456-presentation.mp4
🎙️  Starting transcription job: transcribe-job-1642123456-presentation
✓ Transcription job started successfully
⏳ Polling transcription job status...
🔍 Checking status (attempt 1/120)
⏳ Job still in progress...
⏰ Waiting 5 seconds before next check...
🔍 Checking status (attempt 2/120)
✓ Transcription job completed successfully
🎉 Transcription completed! Result URI: https://...
📥 Retrieving transcription results...
✓ Transcription results retrieved successfully

📝 Transcription Results:
─────────────────────────
Hello everyone, welcome to today's presentation about our quarterly results...
─────────────────────────

🗑️  Cleaning up S3 file: s3://my-bucket/transcribe-temp/1642123456-presentation.mp4
✓ S3 file deleted successfully
```

## Error Handling

The application provides helpful error messages for common issues:

- **File not found**: Checks file path and permissions
- **Unsupported format**: Lists supported file formats
- **AWS credential issues**: Guides through credential setup
- **S3 access problems**: Verifies bucket permissions
- **Transcription failures**: Shows detailed error reasons

## Development

### Project Structure

```
src/
├── main.rs          # Main application logic
├── error.rs         # Error handling (integrated in main.rs)
├── aws.rs           # AWS client management (integrated in main.rs)
├── transcribe.rs    # Transcription logic (integrated in main.rs)
└── utils.rs         # Utility functions (integrated in main.rs)
```

### Dependencies

- **clap** - Command line argument parsing
- **tokio** - Async runtime
- **aws-config** - AWS configuration
- **aws-sdk-s3** - S3 operations
- **aws-sdk-transcribe** - Transcription services
- **thiserror** - Error handling
- **reqwest** - HTTP client for result retrieval
- **serde_json** - JSON parsing

## Troubleshooting

### Common Issues

1. **"AWS credentials not found"**
   - Configure AWS CLI: `aws configure`
   - Set environment variables
   - Check IAM permissions

2. **"S3 bucket access denied"**
   - Verify bucket exists: `aws s3 ls s3://your-bucket`
   - Check bucket permissions
   - Ensure correct region

3. **"File format not supported"**
   - Check supported formats list
   - Verify file extension
   - Try converting file format

4. **"Transcription job failed"**
   - Check file isn't corrupted
   - Verify audio quality
   - Ensure file size < 2GB

### Getting Help

- Check AWS CloudTrail for detailed error logs
- Verify IAM permissions in AWS Console
- Test AWS credentials: `aws sts get-caller-identity`

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with the AWS SDK for Rust
- Uses Amazon Transcribe for speech-to-text conversion
- Inspired by the need for simple, reliable transcription tools
