#!/bin/bash

# Chronicle Release Upload Script
# Uploads release packages to various distribution platforms

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
ROOT_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Configuration
VERBOSE=false
VERSION=""
RELEASE_DIR="$ROOT_DIR/dist"
PLATFORMS=()
DRY_RUN=false
FORCE_UPLOAD=false

# Platform configurations
GITHUB_REPO="${GITHUB_REPO:-}"
GITHUB_TOKEN="${GITHUB_TOKEN:-}"
S3_BUCKET="${S3_BUCKET:-}"
FTP_SERVER="${FTP_SERVER:-}"
FTP_USERNAME="${FTP_USERNAME:-}"
FTP_PASSWORD="${FTP_PASSWORD:-}"

# Logging
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}"
}

# Show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Upload Chronicle release packages to distribution platforms.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    --version VERSION       Release version (required)
    --release-dir DIR       Release directory (default: dist)
    --platform PLATFORM    Upload platform (github, s3, ftp, all)
    --dry-run               Show what would be uploaded without uploading
    --force                 Force upload even if release exists

PLATFORMS:
    github                  GitHub Releases
    s3                      Amazon S3
    ftp                     FTP server
    all                     All configured platforms

ENVIRONMENT VARIABLES:
    GITHUB_REPO             GitHub repository (owner/repo)
    GITHUB_TOKEN            GitHub access token
    S3_BUCKET               S3 bucket name
    FTP_SERVER              FTP server hostname
    FTP_USERNAME            FTP username
    FTP_PASSWORD            FTP password

EXAMPLES:
    $0 --version 1.0.0 --platform github
    $0 --version 1.0.0 --platform all
    $0 --version 1.0.0 --dry-run

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            --version)
                VERSION="$2"
                shift 2
                ;;
            --release-dir)
                RELEASE_DIR="$2"
                shift 2
                ;;
            --platform)
                PLATFORMS+=("$2")
                shift 2
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --force)
                FORCE_UPLOAD=true
                shift
                ;;
            -*)
                error "Unknown option: $1"
                ;;
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done
    
    # Validate required parameters
    if [ -z "$VERSION" ]; then
        error "Version is required. Use --version to specify."
    fi
    
    # If no platforms specified, use all
    if [ ${#PLATFORMS[@]} -eq 0 ]; then
        PLATFORMS=("all")
    fi
    
    # Expand "all" platform
    if [[ " ${PLATFORMS[*]} " =~ " all " ]]; then
        PLATFORMS=("github" "s3" "ftp")
    fi
    
    # Make release directory absolute
    if [[ ! "$RELEASE_DIR" = /* ]]; then
        RELEASE_DIR="$ROOT_DIR/$RELEASE_DIR"
    fi
}

# Check upload prerequisites
check_upload_prerequisites() {
    log "Checking upload prerequisites..."
    
    # Check if release directory exists
    if [ ! -d "$RELEASE_DIR" ]; then
        error "Release directory not found: $RELEASE_DIR"
    fi
    
    # Find release files
    local release_files=()
    while IFS= read -r -d '' file; do
        release_files+=("$file")
    done < <(find "$RELEASE_DIR" -name "*$VERSION*" \( -name "*.dmg" -o -name "*.pkg" -o -name "*.zip" \) -print0)
    
    if [ ${#release_files[@]} -eq 0 ]; then
        error "No release files found for version $VERSION in $RELEASE_DIR"
    fi
    
    info "Found ${#release_files[@]} release files for version $VERSION"
    
    # Check platform-specific prerequisites
    for platform in "${PLATFORMS[@]}"; do
        check_platform_prerequisites "$platform"
    done
    
    log "Prerequisites check completed"
}

# Check platform-specific prerequisites
check_platform_prerequisites() {
    local platform="$1"
    
    case $platform in
        "github")
            if [ -z "$GITHUB_REPO" ]; then
                error "GITHUB_REPO environment variable not set"
            fi
            
            if [ -z "$GITHUB_TOKEN" ]; then
                error "GITHUB_TOKEN environment variable not set"
            fi
            
            if ! command -v gh &> /dev/null; then
                error "GitHub CLI (gh) not found. Install with: brew install gh"
            fi
            ;;
        "s3")
            if [ -z "$S3_BUCKET" ]; then
                error "S3_BUCKET environment variable not set"
            fi
            
            if ! command -v aws &> /dev/null; then
                error "AWS CLI not found. Install with: brew install awscli"
            fi
            
            # Check AWS credentials
            if ! aws sts get-caller-identity &> /dev/null; then
                error "AWS credentials not configured. Run: aws configure"
            fi
            ;;
        "ftp")
            if [ -z "$FTP_SERVER" ] || [ -z "$FTP_USERNAME" ] || [ -z "$FTP_PASSWORD" ]; then
                error "FTP configuration incomplete. Set FTP_SERVER, FTP_USERNAME, FTP_PASSWORD"
            fi
            
            if ! command -v lftp &> /dev/null; then
                error "lftp not found. Install with: brew install lftp"
            fi
            ;;
    esac
}

# Find release files
find_release_files() {
    local release_files=()
    
    # Find all release files for this version
    while IFS= read -r -d '' file; do
        release_files+=("$file")
    done < <(find "$RELEASE_DIR" -name "*$VERSION*" \( -name "*.dmg" -o -name "*.pkg" -o -name "*.zip" \) -print0)
    
    # Also include info files
    while IFS= read -r -d '' file; do
        release_files+=("$file")
    done < <(find "$RELEASE_DIR" -name "*$VERSION*" -name "*info.txt" -print0)
    
    printf '%s\n' "${release_files[@]}"
}

# Upload to GitHub Releases
upload_to_github() {
    log "Uploading to GitHub Releases..."
    
    local release_files
    mapfile -t release_files < <(find_release_files)
    
    if [ ${#release_files[@]} -eq 0 ]; then
        warn "No release files found for GitHub upload"
        return 0
    fi
    
    # Check if release exists
    local release_exists=false
    if gh release view "v$VERSION" --repo "$GITHUB_REPO" &> /dev/null; then
        release_exists=true
        if [ "$FORCE_UPLOAD" = false ]; then
            warn "GitHub release v$VERSION already exists. Use --force to overwrite."
            return 0
        fi
    fi
    
    if [ "$DRY_RUN" = true ]; then
        info "DRY RUN: Would upload to GitHub:"
        for file in "${release_files[@]}"; do
            info "  $(basename "$file")"
        done
        return 0
    fi
    
    # Create or update release
    if [ "$release_exists" = true ]; then
        log "Updating existing GitHub release..."
        
        # Delete existing assets
        gh release delete-asset "v$VERSION" --repo "$GITHUB_REPO" "*" --yes || true
    else
        log "Creating new GitHub release..."
        
        # Create release notes
        local release_notes="Release notes for Chronicle v$VERSION"
        if [ -f "$ROOT_DIR/CHANGELOG.md" ]; then
            # Extract release notes from changelog
            release_notes=$(awk "/## \[$VERSION\]/,/## \[/{if(/## \[/ && !/## \[$VERSION\]/) exit; if(!/## \[$VERSION\]/) print}" "$ROOT_DIR/CHANGELOG.md" || echo "Release notes for Chronicle v$VERSION")
        fi
        
        # Create release
        gh release create "v$VERSION" \
           --repo "$GITHUB_REPO" \
           --title "Chronicle v$VERSION" \
           --notes "$release_notes"
    fi
    
    # Upload assets
    for file in "${release_files[@]}"; do
        log "Uploading $(basename "$file")..."
        
        if ! gh release upload "v$VERSION" "$file" --repo "$GITHUB_REPO"; then
            error "Failed to upload $(basename "$file") to GitHub"
        fi
    done
    
    log "GitHub upload completed"
}

# Upload to S3
upload_to_s3() {
    log "Uploading to S3..."
    
    local release_files
    mapfile -t release_files < <(find_release_files)
    
    if [ ${#release_files[@]} -eq 0 ]; then
        warn "No release files found for S3 upload"
        return 0
    fi
    
    local s3_prefix="releases/v$VERSION"
    
    if [ "$DRY_RUN" = true ]; then
        info "DRY RUN: Would upload to S3 bucket $S3_BUCKET:"
        for file in "${release_files[@]}"; do
            info "  s3://$S3_BUCKET/$s3_prefix/$(basename "$file")"
        done
        return 0
    fi
    
    # Upload files
    for file in "${release_files[@]}"; do
        local filename=$(basename "$file")
        local s3_key="$s3_prefix/$filename"
        
        log "Uploading $filename to S3..."
        
        local aws_flags=""
        if [ "$VERBOSE" = false ]; then
            aws_flags="--no-progress"
        fi
        
        if ! aws s3 cp "$file" "s3://$S3_BUCKET/$s3_key" $aws_flags; then
            error "Failed to upload $filename to S3"
        fi
    done
    
    # Create latest symlinks
    log "Creating latest version symlinks..."
    for file in "${release_files[@]}"; do
        local filename=$(basename "$file")
        local latest_filename="${filename/$VERSION/latest}"
        local s3_key="$s3_prefix/$filename"
        local latest_key="releases/latest/$latest_filename"
        
        # Copy to latest directory
        aws s3 cp "s3://$S3_BUCKET/$s3_key" "s3://$S3_BUCKET/$latest_key" --no-progress || true
    done
    
    log "S3 upload completed"
}

# Upload to FTP
upload_to_ftp() {
    log "Uploading to FTP..."
    
    local release_files
    mapfile -t release_files < <(find_release_files)
    
    if [ ${#release_files[@]} -eq 0 ]; then
        warn "No release files found for FTP upload"
        return 0
    fi
    
    local ftp_dir="chronicle/releases/v$VERSION"
    
    if [ "$DRY_RUN" = true ]; then
        info "DRY RUN: Would upload to FTP server $FTP_SERVER:"
        for file in "${release_files[@]}"; do
            info "  $ftp_dir/$(basename "$file")"
        done
        return 0
    fi
    
    # Create FTP script
    local ftp_script="$RELEASE_DIR/ftp_upload.lftp"
    
    cat > "$ftp_script" << EOF
set ftp:ssl-allow no
set ftp:ssl-force no
set ssl:verify-certificate no

open ftp://$FTP_USERNAME:$FTP_PASSWORD@$FTP_SERVER

mkdir -p $ftp_dir
cd $ftp_dir

EOF
    
    # Add upload commands
    for file in "${release_files[@]}"; do
        echo "put \"$file\"" >> "$ftp_script"
    done
    
    echo "quit" >> "$ftp_script"
    
    # Execute FTP upload
    log "Connecting to FTP server..."
    
    if ! lftp -f "$ftp_script"; then
        error "FTP upload failed"
    fi
    
    # Clean up FTP script
    rm -f "$ftp_script"
    
    log "FTP upload completed"
}

# Generate upload summary
generate_upload_summary() {
    log "Generating upload summary..."
    
    local summary_file="$RELEASE_DIR/upload_summary_v$VERSION.txt"
    
    cat > "$summary_file" << EOF
Chronicle Release Upload Summary
Version: $VERSION
Upload Date: $(date)
Platforms: ${PLATFORMS[*]}
Dry Run: $DRY_RUN

Release Files:
EOF
    
    local release_files
    mapfile -t release_files < <(find_release_files)
    
    for file in "${release_files[@]}"; do
        local filename=$(basename "$file")
        local filesize=$(ls -lh "$file" | awk '{print $5}')
        echo "  $filename ($filesize)" >> "$summary_file"
    done
    
    echo "" >> "$summary_file"
    echo "Upload Results:" >> "$summary_file"
    
    for platform in "${PLATFORMS[@]}"; do
        echo "  $platform: COMPLETED" >> "$summary_file"
    done
    
    # Add platform-specific URLs
    echo "" >> "$summary_file"
    echo "Download URLs:" >> "$summary_file"
    
    for platform in "${PLATFORMS[@]}"; do
        case $platform in
            "github")
                if [ -n "$GITHUB_REPO" ]; then
                    echo "  GitHub: https://github.com/$GITHUB_REPO/releases/tag/v$VERSION" >> "$summary_file"
                fi
                ;;
            "s3")
                if [ -n "$S3_BUCKET" ]; then
                    echo "  S3: https://$S3_BUCKET.s3.amazonaws.com/releases/v$VERSION/" >> "$summary_file"
                fi
                ;;
            "ftp")
                if [ -n "$FTP_SERVER" ]; then
                    echo "  FTP: ftp://$FTP_SERVER/chronicle/releases/v$VERSION/" >> "$summary_file"
                fi
                ;;
        esac
    done
    
    log "Upload summary saved to: $summary_file"
}

# Upload to platform
upload_to_platform() {
    local platform="$1"
    
    case $platform in
        "github")
            upload_to_github
            ;;
        "s3")
            upload_to_s3
            ;;
        "ftp")
            upload_to_ftp
            ;;
        *)
            error "Unknown platform: $platform"
            ;;
    esac
}

# Upload to all platforms
upload_to_all_platforms() {
    log "Uploading to platforms: ${PLATFORMS[*]}"
    
    local failed_platforms=()
    
    for platform in "${PLATFORMS[@]}"; do
        if ! upload_to_platform "$platform"; then
            failed_platforms+=("$platform")
        fi
    done
    
    if [ ${#failed_platforms[@]} -gt 0 ]; then
        error "Upload failed for platforms: ${failed_platforms[*]}"
    fi
}

# Main upload function
main() {
    log "Starting Chronicle release upload..."
    
    parse_args "$@"
    
    info "Upload configuration:"
    info "  Version: $VERSION"
    info "  Release Directory: $RELEASE_DIR"
    info "  Platforms: ${PLATFORMS[*]}"
    info "  Dry Run: $DRY_RUN"
    info "  Force Upload: $FORCE_UPLOAD"
    
    check_upload_prerequisites
    upload_to_all_platforms
    generate_upload_summary
    
    log "Release upload completed successfully!"
    
    if [ "$DRY_RUN" = false ]; then
        info "Chronicle v$VERSION has been uploaded to: ${PLATFORMS[*]}"
    else
        info "Dry run completed - no files were actually uploaded"
    fi
}

# Run main function
main "$@"