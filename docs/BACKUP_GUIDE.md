# Chronicle Backup Guide

This guide covers Chronicle's comprehensive backup features, including local backups, auto-backup to external drives, and privacy-preserving cloud backup.

## Table of Contents

- [Overview](#overview)
- [Local Backup](#local-backup)
- [Auto-Backup to External Drives](#auto-backup-to-external-drives)
- [Cloud Backup (S3)](#cloud-backup-s3)
- [CLI Commands](#cli-commands)
- [Configuration](#configuration)
- [Security and Privacy](#security-and-privacy)
- [Troubleshooting](#troubleshooting)

## Overview

Chronicle provides three types of backup functionality:

1. **Local Backup**: Traditional backup to local or network storage
2. **Auto-Backup**: Automatic backup when specific external drives are connected
3. **Cloud Backup**: Privacy-preserving backup to AWS S3 with client-side encryption

All backup methods maintain Chronicle's privacy-first approach with optional encryption and compression.

## Local Backup

### Basic Local Backup

Create a simple backup to a local directory:

```bash
chronictl backup --destination /path/to/backup/chronicle-backup.tar.gz
```

### Advanced Local Backup Options

```bash
chronictl backup \
  --destination /backups/chronicle-$(date +%Y%m%d).tar.gz \
  --compression gzip \
  --encryption \
  --verify \
  --include-metadata \
  --progress
```

### Options

- `--destination`: Backup destination path (required)
- `--compression`: Compression format (`gzip`, `bzip2`, `lz4`)
- `--encryption`: Enable encryption (will prompt for password)
- `--verify`: Verify backup integrity after creation
- `--include-metadata`: Include system metadata in backup
- `--progress`: Show progress during backup
- `--overwrite`: Overwrite existing backup files
- `--timeout`: Backup timeout in seconds (default: 3600)
- `--time`: Backup specific time range
- `--event-types`: Backup only specific event types
- `--dry-run`: Show what would be backed up without creating backup

## Auto-Backup to External Drives

Auto-backup automatically creates backups when specific external drives are connected to your system.

### Enabling Auto-Backup

```bash
# Configure auto-backup for a specific drive UUID
chronictl backup \
  --auto-backup \
  --target-drive "12345678-1234-1234-1234-123456789ABC" \
  --drive-id-type uuid \
  --destination /Chronicle

# Configure auto-backup for a drive by volume label
chronictl backup \
  --auto-backup \
  --target-drive "MyBackupDrive" \
  --drive-id-type volume_label
```

### Drive Identification Methods

Auto-backup supports multiple ways to identify target drives:

#### UUID (Recommended)
Most reliable method using the drive's unique identifier:
```bash
--target-drive "12345678-1234-1234-1234-123456789ABC" --drive-id-type uuid
```

#### Volume Label
Identifies drives by their user-assigned label:
```bash
--target-drive "BackupDrive" --drive-id-type volume_label
```

#### Serial Number
Hardware-level identification:
```bash
--target-drive "WD1234567890" --drive-id-type serial_number
```

### Configuration File Setup

Add auto-backup configuration to `~/.config/chronicle/config.toml`:

```toml
[auto_backup]
enabled = true
remove_local_after_backup = false
verification_required = true
backup_destination_path = "/Chronicle"
encryption_enabled = true
compression_enabled = true
retry_attempts = 3
retry_delay_seconds = 60

# Multiple target drives can be configured
[[auto_backup.target_drives]]
type = "uuid"
identifier = "12345678-1234-1234-1234-123456789ABC"

[[auto_backup.target_drives]]
type = "volume_label"
identifier = "MyBackupDrive"

[[auto_backup.target_drives]]
type = "serial_number"
identifier = "WD1234567890"
```

### Safety Features

#### Remove Local Files Option
⚠️ **WARNING**: The `--remove-local` option permanently deletes local files after successful backup.

```bash
chronictl backup \
  --auto-backup \
  --target-drive "BackupDrive" \
  --remove-local  # DANGEROUS - will prompt for confirmation
```

This option:
- Requires explicit confirmation
- Only removes files after successful backup verification
- Cannot be undone
- Should only be used when you're certain the backup is reliable

## Cloud Backup (S3)

Chronicle's cloud backup maintains privacy through client-side encryption while providing convenient cloud storage.

### Basic Cloud Backup

```bash
chronictl backup \
  --cloud \
  --s3-uri s3://my-backup-bucket/chronicle-data \
  --destination /local/backup/path
```

### Continuous Cloud Backup

For real-time cloud synchronization:

```bash
chronictl backup \
  --cloud \
  --s3-uri s3://my-backup-bucket/chronicle-data \
  --continuous \
  --destination /local/backup/path
```

### Cloud Backup Configuration

Configure AWS S3 backup in `~/.config/chronicle/config.toml`:

```toml
[cloud_backup]
enabled = true
provider = "s3"
continuous_backup = false
schedule = "daily"  # realtime, hourly, daily, weekly, monthly
encryption_enabled = true
client_side_encryption = true  # Privacy-first: data encrypted before upload
retention_days = 90
max_backup_size_gb = 10
compression_enabled = true

[cloud_backup.s3]
bucket_name = "my-chronicle-backups"
region = "us-west-2"
prefix = "chronicle-data"
storage_class = "STANDARD_IA"
server_side_encryption = true

# AWS credentials (optional - can use IAM roles or environment variables)
# access_key_id = "AKIAIOSFODNN7EXAMPLE"
# secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
```

### Privacy Features

Chronicle's cloud backup prioritizes privacy:

1. **Client-Side Encryption**: Data is encrypted on your device before upload
2. **Zero-Knowledge**: Cloud provider cannot decrypt your data
3. **Compression**: Obfuscates data patterns before encryption
4. **Configurable Retention**: Automatic cleanup of old backups
5. **Local-First**: Cloud backup supplements, never replaces local data

### AWS Setup

#### 1. Create S3 Bucket
```bash
aws s3 mb s3://my-chronicle-backups --region us-west-2
```

#### 2. Configure IAM Policy
Create an IAM policy with minimal required permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:PutObjectAcl",
        "s3:GetObject",
        "s3:DeleteObject"
      ],
      "Resource": "arn:aws:s3:::my-chronicle-backups/*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "s3:ListBucket"
      ],
      "Resource": "arn:aws:s3:::my-chronicle-backups"
    }
  ]
}
```

#### 3. Configure AWS Credentials

Option 1: AWS CLI
```bash
aws configure
```

Option 2: Environment Variables
```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_DEFAULT_REGION=us-west-2
```

Option 3: IAM Roles (recommended for EC2)
No credentials needed when using IAM instance profiles.

## CLI Commands

### Backup Command Reference

```bash
chronictl backup [OPTIONS]
```

#### Core Options
- `--destination <PATH>`: Backup destination path (required)
- `--include-metadata`: Include metadata in backup
- `--compression <FORMAT>`: Compression (`gzip`, `bzip2`, `lz4`)
- `--encryption [PASSWORD]`: Enable encryption (prompts if no password)
- `--overwrite`: Overwrite existing backup
- `--verify`: Verify backup integrity
- `--progress`: Show progress during backup
- `--timeout <SECONDS>`: Backup timeout (default: 3600)
- `--dry-run`: Show backup plan without executing

#### Filtering Options
- `--time <RANGE>`: Backup specific time range
  - `last-hour`, `last-day`, `last-week`, `last-month`
  - `today`, `yesterday`
  - `2024-01-01..2024-01-31` (date range)
- `--event-types <TYPES>`: Comma-separated event types
  - `screen_capture,file_system,network`

#### Cloud Backup Options
- `--cloud`: Enable cloud backup
- `--s3-uri <URI>`: S3 bucket URI (`s3://bucket/prefix`)
- `--continuous`: Enable continuous backup

#### Auto-Backup Options
- `--auto-backup`: Enable auto-backup
- `--target-drive <ID>`: Target drive identifier
- `--drive-id-type <TYPE>`: ID type (`uuid`, `volume_label`, `serial_number`)
- `--remove-local`: Remove local files after backup (dangerous)

### Example Commands

#### Complete Local Backup
```bash
chronictl backup \
  --destination ~/backups/chronicle-$(date +%Y%m%d).tar.gz \
  --compression gzip \
  --encryption \
  --verify \
  --progress \
  --include-metadata
```

#### Cloud + Auto-Backup Combo
```bash
chronictl backup \
  --destination ~/backups/chronicle-local.tar.gz \
  --cloud \
  --s3-uri s3://my-backups/chronicle \
  --auto-backup \
  --target-drive "BackupDrive" \
  --drive-id-type volume_label \
  --compression lz4 \
  --verify
```

#### Time-Filtered Backup
```bash
chronictl backup \
  --destination ~/backups/last-week.tar.gz \
  --time last-week \
  --event-types screen_capture,file_system \
  --compression bzip2
```

## Configuration

### Complete Configuration Reference

```toml
[auto_backup]
enabled = false
remove_local_after_backup = false
verification_required = true
backup_destination_path = "/Chronicle"
encryption_enabled = true
compression_enabled = true
retry_attempts = 3
retry_delay_seconds = 60

[[auto_backup.target_drives]]
type = "uuid"
identifier = "12345678-1234-1234-1234-123456789ABC"

[cloud_backup]
enabled = false
provider = "s3"
continuous_backup = false
schedule = "daily"
encryption_enabled = true
client_side_encryption = true
retention_days = 90
max_backup_size_gb = 10
compression_enabled = true

[cloud_backup.s3]
bucket_name = "my-chronicle-backups"
region = "us-west-2"
prefix = "chronicle-data"
storage_class = "STANDARD_IA"
server_side_encryption = true

[drive_monitoring]
enabled = true
monitor_all_drives = true
notify_on_connection = true
log_drive_events = true
sample_rate = 1.0
monitor_usb_drives = true
monitor_thunderbolt_drives = true
monitor_firewire_drives = true
monitor_internal_drives = false
```

### Environment Variables

Chronicle respects these environment variables for cloud backup:

```bash
# AWS Configuration
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key
AWS_DEFAULT_REGION=us-west-2
AWS_PROFILE=chronicle

# Chronicle Configuration
CHRONICLE_CLOUD_BACKUP_ENABLED=true
CHRONICLE_S3_BUCKET=my-backups
CHRONICLE_AUTO_BACKUP_ENABLED=true
```

## Security and Privacy

### Encryption

Chronicle provides multiple layers of encryption:

1. **Local Encryption**: AES-256-GCM for local data at rest
2. **Client-Side Encryption**: Data encrypted before cloud upload
3. **Server-Side Encryption**: Additional S3 server-side encryption
4. **Transport Encryption**: HTTPS/TLS for all cloud communication

### Key Management

- **Local Keys**: Stored securely in macOS Keychain
- **Cloud Keys**: Separate encryption keys for cloud data
- **Key Rotation**: Automatic key rotation (configurable)
- **Zero-Knowledge**: Cloud provider cannot access encryption keys

### Privacy Best Practices

1. **Enable Client-Side Encryption**: Always encrypt before cloud upload
2. **Use Unique Buckets**: Don't share S3 buckets with other applications
3. **Configure Retention**: Automatically delete old backups
4. **Monitor Access**: Use AWS CloudTrail to monitor bucket access
5. **Verify Backups**: Always verify backup integrity
6. **Test Recovery**: Regularly test backup restoration

### Compliance Considerations

Chronicle's backup features support various compliance requirements:

- **GDPR**: Client-side encryption ensures data controller privacy
- **HIPAA**: Encryption and access controls meet security requirements
- **SOC 2**: Audit logging and monitoring support compliance
- **Data Residency**: Configurable AWS regions for data location requirements

## Troubleshooting

### Common Issues

#### Auto-Backup Not Triggering

1. **Check Drive Detection**:
   ```bash
   # Check if drive is detected
   diskutil list external
   
   # Check Chronicle drive monitoring
   tail -f ~/.config/chronicle/logs/chronicle.log | grep drive
   ```

2. **Verify Drive Identifier**:
   ```bash
   # Get drive UUID
   diskutil info /dev/disk2 | grep UUID
   
   # Get volume label
   diskutil info /dev/disk2 | grep "Volume Name"
   ```

3. **Check Configuration**:
   ```bash
   # Verify configuration
   chronictl config get auto_backup
   ```

#### Cloud Backup Upload Failures

1. **Check AWS Credentials**:
   ```bash
   aws sts get-caller-identity
   ```

2. **Verify S3 Permissions**:
   ```bash
   aws s3 ls s3://your-bucket-name/
   ```

3. **Check Network Connectivity**:
   ```bash
   ping s3.amazonaws.com
   ```

4. **Review Logs**:
   ```bash
   tail -f ~/.config/chronicle/logs/chronicle.log | grep cloud_backup
   ```

#### Backup Verification Failures

1. **Check File Integrity**:
   ```bash
   # Verify backup file
   chronictl backup verify /path/to/backup.tar.gz
   ```

2. **Check Available Space**:
   ```bash
   df -h /path/to/backup/destination
   ```

3. **Review Backup Logs**:
   ```bash
   chronictl status --detailed
   ```

### Error Messages

#### "Permission denied for drive monitoring"
- Grant Chronicle accessibility permissions in System Preferences
- Restart Chronicle after granting permissions

#### "S3 upload failed: Access Denied"
- Check AWS credentials configuration
- Verify S3 bucket permissions
- Ensure bucket exists and is accessible

#### "Backup verification failed"
- Check backup file isn't corrupted
- Verify destination has sufficient space
- Ensure encryption password is correct

#### "Target drive not found"
- Verify drive is properly connected
- Check drive identifier in configuration
- Ensure drive is mounted and accessible

### Performance Optimization

#### Large Backup Optimization

1. **Use Compression**: Reduces backup size and upload time
2. **Filter Event Types**: Backup only necessary data
3. **Time Range Filtering**: Backup specific periods
4. **Continuous vs Scheduled**: Choose appropriate backup frequency

#### Cloud Upload Optimization

1. **Choose Appropriate Storage Class**:
   - `STANDARD`: Frequent access
   - `STANDARD_IA`: Infrequent access (recommended)
   - `GLACIER`: Archive storage

2. **Configure Upload Concurrency**:
   ```toml
   [cloud_backup]
   max_concurrent_uploads = 3
   upload_part_size_mb = 10
   ```

3. **Monitor Upload Progress**:
   ```bash
   chronictl backup --cloud --progress
   ```

### Recovery Procedures

#### Restore from Local Backup

```bash
chronictl restore --backup /path/to/backup.tar.gz --destination /restore/path
```

#### Restore from S3

```bash
# Download from S3
aws s3 cp s3://my-backups/chronicle/backup.tar.gz ./backup.tar.gz

# Restore locally
chronictl restore --backup ./backup.tar.gz --destination /restore/path
```

#### Emergency Recovery

If Chronicle is completely unavailable:

1. **Manual S3 Download**:
   ```bash
   aws s3 sync s3://my-backups/chronicle/ ./emergency-restore/
   ```

2. **Extract Backups**:
   ```bash
   tar -xzf backup.tar.gz -C ./restored-data/
   ```

3. **Decrypt if Necessary**:
   ```bash
   # Use Chronicle's decryption tools or standard tools
   gpg --decrypt encrypted-backup.gpg > decrypted-backup.tar.gz
   ```

## Support and Resources

### Documentation
- [Chronicle User Guide](USER_GUIDE.md)
- [Configuration Reference](CONFIGURATION.md)
- [Security Guide](SECURITY.md)
- [Troubleshooting Guide](TROUBLESHOOTING.md)

### Getting Help
- [GitHub Issues](https://github.com/chronicle/chronicle/issues)
- [Community Discord](https://discord.gg/chronicle)
- Email: support@chronicle.dev

### Best Practices
- Test backup and restore procedures regularly
- Monitor backup success and failure rates
- Keep backup configurations under version control
- Document custom backup procedures for your organization
- Review and update retention policies periodically

---

*Chronicle's backup features are designed to provide comprehensive data protection while maintaining your privacy and control over your personal data.*