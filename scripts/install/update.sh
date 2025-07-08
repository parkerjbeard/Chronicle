#!/bin/bash

# Chronicle Update Script
# Updates existing Chronicle installation

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VERBOSE=false
FORCE_UPDATE=false
BACKUP_CONFIG=true
AUTO_RESTART=false
UPDATE_SOURCE=""
CHECK_ONLY=false
DOWNLOAD_URL=""

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

Update Chronicle to the latest version.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -f, --force             Force update even if versions match
    --check                 Check for updates without installing
    --no-backup             Don't backup configuration
    --auto-restart          Automatically restart services after update
    --source SOURCE         Update source (auto, local, github, url)
    --url URL               Custom download URL
    --local PATH            Use local source directory

UPDATE SOURCES:
    auto                    Auto-detect best source (default)
    local                   Use local source directory
    github                  Download from GitHub releases
    url                     Download from custom URL

EXAMPLES:
    $0                      # Auto-update to latest version
    $0 --check              # Check for updates only
    $0 --source github      # Update from GitHub releases
    $0 --local /path/to/chronicle  # Update from local source

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
            -f|--force)
                FORCE_UPDATE=true
                shift
                ;;
            --check)
                CHECK_ONLY=true
                shift
                ;;
            --no-backup)
                BACKUP_CONFIG=false
                shift
                ;;
            --auto-restart)
                AUTO_RESTART=true
                shift
                ;;
            --source)
                UPDATE_SOURCE="$2"
                shift 2
                ;;
            --url)
                DOWNLOAD_URL="$2"
                UPDATE_SOURCE="url"
                shift 2
                ;;
            --local)
                UPDATE_SOURCE="local"
                LOCAL_SOURCE="$2"
                shift 2
                ;;
            -*)
                error "Unknown option: $1"
                ;;
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done
    
    # Default to auto source
    if [ -z "$UPDATE_SOURCE" ]; then
        UPDATE_SOURCE="auto"
    fi
    
    # Validate update source
    case $UPDATE_SOURCE in
        "auto"|"local"|"github"|"url")
            ;;
        *)
            error "Invalid update source: $UPDATE_SOURCE"
            ;;
    esac
}

# Detect current installation
detect_current_installation() {
    log "Detecting current Chronicle installation..."
    
    # Check for system installation
    if [ -f "/usr/local/bin/chronicle" ]; then
        CURRENT_INSTALL_TYPE="system"
        CURRENT_BIN_DIR="/usr/local/bin"
        CURRENT_CONFIG_DIR="/usr/local/etc/chronicle"
        CURRENT_DATA_DIR="/usr/local/share/chronicle"
        info "System installation detected"
        
        # Check if we need root privileges
        if [ "$EUID" -ne 0 ]; then
            warn "System installation detected but not running as root"
            warn "Some operations may require sudo privileges"
        fi
    
    # Check for local installation
    elif [ -f "$HOME/.local/bin/chronicle" ]; then
        CURRENT_INSTALL_TYPE="local"
        CURRENT_BIN_DIR="$HOME/.local/bin"
        CURRENT_CONFIG_DIR="$HOME/.config/chronicle"
        CURRENT_DATA_DIR="$HOME/.local/share/chronicle"
        info "Local installation detected"
    
    else
        error "No Chronicle installation found"
    fi
    
    # Get current version
    if [ -f "$CURRENT_BIN_DIR/chronicle" ]; then
        CURRENT_VERSION=$("$CURRENT_BIN_DIR/chronicle" --version 2>/dev/null | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' || echo "unknown")
        info "Current version: $CURRENT_VERSION"
    else
        CURRENT_VERSION="unknown"
        warn "Cannot determine current version"
    fi
}

# Check for available updates
check_for_updates() {
    log "Checking for available updates..."
    
    case $UPDATE_SOURCE in
        "auto"|"github")
            check_github_releases
            ;;
        "local")
            check_local_version
            ;;
        "url")
            check_url_version
            ;;
    esac
}

# Check GitHub releases
check_github_releases() {
    log "Checking GitHub releases..."
    
    # Check if GitHub CLI is available
    if command -v gh &> /dev/null; then
        LATEST_VERSION=$(gh release view --repo "chronicle-rs/chronicle" --json tagName --jq '.tagName' 2>/dev/null | sed 's/^v//' || echo "")
    elif command -v curl &> /dev/null; then
        # Use GitHub API directly
        LATEST_VERSION=$(curl -s https://api.github.com/repos/chronicle-rs/chronicle/releases/latest | grep '"tag_name"' | sed 's/.*"v\([^"]*\)".*/\1/' || echo "")
    else
        error "No tool available to check GitHub releases (need gh or curl)"
    fi
    
    if [ -z "$LATEST_VERSION" ]; then
        error "Failed to get latest version from GitHub"
    fi
    
    info "Latest version: $LATEST_VERSION"
    
    if [ "$CURRENT_VERSION" = "$LATEST_VERSION" ] && [ "$FORCE_UPDATE" = false ]; then
        info "Already running the latest version"
        if [ "$CHECK_ONLY" = true ]; then
            exit 0
        fi
        
        read -p "Update anyway? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            info "Update cancelled"
            exit 0
        fi
    fi
}

# Check local version
check_local_version() {
    if [ -z "${LOCAL_SOURCE:-}" ]; then
        error "Local source path not specified. Use --local PATH"
    fi
    
    if [ ! -d "$LOCAL_SOURCE" ]; then
        error "Local source directory not found: $LOCAL_SOURCE"
    fi
    
    log "Checking local source version..."
    
    # Try to get version from Cargo.toml
    if [ -f "$LOCAL_SOURCE/Cargo.toml" ]; then
        LATEST_VERSION=$(grep '^version = ' "$LOCAL_SOURCE/Cargo.toml" | head -1 | sed 's/.*"\([^"]*\)".*/\1/')
    elif [ -f "$LOCAL_SOURCE/cli/Cargo.toml" ]; then
        LATEST_VERSION=$(grep '^version = ' "$LOCAL_SOURCE/cli/Cargo.toml" | head -1 | sed 's/.*"\([^"]*\)".*/\1/')
    else
        LATEST_VERSION="local-build"
    fi
    
    info "Local source version: $LATEST_VERSION"
}

# Check URL version
check_url_version() {
    if [ -z "$DOWNLOAD_URL" ]; then
        error "Download URL not specified. Use --url URL"
    fi
    
    log "Using custom download URL..."
    LATEST_VERSION="custom"
    info "Custom download will be performed"
}

# Backup current installation
backup_current_installation() {
    if [ "$BACKUP_CONFIG" = false ]; then
        return 0
    fi
    
    log "Backing up current installation..."
    
    local backup_dir="/tmp/chronicle_backup_$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$backup_dir"
    
    # Backup configuration
    if [ -d "$CURRENT_CONFIG_DIR" ]; then
        cp -R "$CURRENT_CONFIG_DIR" "$backup_dir/config"
        info "Configuration backed up to: $backup_dir/config"
    fi
    
    # Backup current binaries
    if [ -f "$CURRENT_BIN_DIR/chronicle" ]; then
        mkdir -p "$backup_dir/bin"
        cp "$CURRENT_BIN_DIR/chronicle" "$backup_dir/bin/"
        if [ -f "$CURRENT_BIN_DIR/chronicle-packer" ]; then
            cp "$CURRENT_BIN_DIR/chronicle-packer" "$backup_dir/bin/"
        fi
        info "Binaries backed up to: $backup_dir/bin"
    fi
    
    echo "$backup_dir" > "/tmp/chronicle_last_backup"
    
    log "Backup completed: $backup_dir"
}

# Stop Chronicle services
stop_chronicle_services() {
    log "Stopping Chronicle services..."
    
    # Stop daemon if running
    if [ -f "/Library/LaunchDaemons/com.chronicle.daemon.plist" ]; then
        log "Stopping system daemon..."
        launchctl stop com.chronicle.daemon 2>/dev/null || true
    fi
    
    # Kill any running Chronicle processes
    pkill -f "chronicle" 2>/dev/null || true
    pkill -f "ChronicleUI" 2>/dev/null || true
    
    # Wait for processes to stop
    sleep 3
    
    log "Chronicle services stopped"
}

# Download update from GitHub
download_github_update() {
    log "Downloading update from GitHub..."
    
    local download_dir="/tmp/chronicle_update_$$"
    mkdir -p "$download_dir"
    
    # Determine download URL
    local asset_name=""
    case "$(uname -m)" in
        "x86_64")
            asset_name="Chronicle-$LATEST_VERSION-universal.zip"
            ;;
        "arm64")
            asset_name="Chronicle-$LATEST_VERSION-universal.zip"
            ;;
        *)
            asset_name="Chronicle-$LATEST_VERSION-full.zip"
            ;;
    esac
    
    if command -v gh &> /dev/null; then
        # Use GitHub CLI
        cd "$download_dir"
        if ! gh release download "v$LATEST_VERSION" --repo "chronicle-rs/chronicle" --pattern "$asset_name"; then
            error "Failed to download release from GitHub"
        fi
    elif command -v curl &> /dev/null; then
        # Use curl with GitHub API
        local download_url="https://github.com/chronicle-rs/chronicle/releases/download/v$LATEST_VERSION/$asset_name"
        if ! curl -L -o "$download_dir/$asset_name" "$download_url"; then
            error "Failed to download release from GitHub"
        fi
    else
        error "No download tool available (need gh or curl)"
    fi
    
    # Extract the archive
    cd "$download_dir"
    if ! unzip -q "$asset_name"; then
        error "Failed to extract downloaded archive"
    fi
    
    # Find extracted directory
    local extracted_dir=$(find . -maxdepth 1 -type d -name "Chronicle-*" | head -1)
    if [ -z "$extracted_dir" ]; then
        error "Could not find extracted directory"
    fi
    
    UPDATE_SOURCE_DIR="$download_dir/$extracted_dir"
    log "Update downloaded and extracted to: $UPDATE_SOURCE_DIR"
}

# Download update from URL
download_url_update() {
    log "Downloading update from URL..."
    
    local download_dir="/tmp/chronicle_update_$$"
    mkdir -p "$download_dir"
    
    local filename=$(basename "$DOWNLOAD_URL")
    
    if ! curl -L -o "$download_dir/$filename" "$DOWNLOAD_URL"; then
        error "Failed to download from URL: $DOWNLOAD_URL"
    fi
    
    # Extract if it's an archive
    cd "$download_dir"
    case "$filename" in
        *.zip)
            if ! unzip -q "$filename"; then
                error "Failed to extract ZIP archive"
            fi
            ;;
        *.tar.gz|*.tgz)
            if ! tar -xzf "$filename"; then
                error "Failed to extract tar.gz archive"
            fi
            ;;
        *)
            error "Unsupported file format: $filename"
            ;;
    esac
    
    # Find extracted directory
    local extracted_dir=$(find . -maxdepth 1 -type d -name "Chronicle-*" | head -1)
    if [ -z "$extracted_dir" ]; then
        error "Could not find extracted directory"
    fi
    
    UPDATE_SOURCE_DIR="$download_dir/$extracted_dir"
    log "Update downloaded and extracted to: $UPDATE_SOURCE_DIR"
}

# Prepare local update
prepare_local_update() {
    log "Preparing local update..."
    
    UPDATE_SOURCE_DIR="$LOCAL_SOURCE"
    
    # Build if needed
    if [ ! -f "$UPDATE_SOURCE_DIR/cli/target/release/chronicle" ] || \
       [ ! -f "$UPDATE_SOURCE_DIR/packer/target/release/chronicle-packer" ]; then
        log "Building Chronicle from source..."
        
        cd "$UPDATE_SOURCE_DIR"
        
        if [ -f "scripts/dev/build.sh" ]; then
            if ! ./scripts/dev/build.sh --release; then
                error "Failed to build Chronicle from source"
            fi
        else
            # Manual build
            cd cli && cargo build --release && cd ..
            cd packer && cargo build --release && cd ..
        fi
    fi
    
    log "Local update prepared"
}

# Install update
install_update() {
    log "Installing update..."
    
    case $UPDATE_SOURCE in
        "github")
            install_from_package
            ;;
        "url")
            install_from_package
            ;;
        "local")
            install_from_source
            ;;
    esac
}

# Install from downloaded package
install_from_package() {
    log "Installing from package..."
    
    # Check if package has install script
    if [ -f "$UPDATE_SOURCE_DIR/install.sh" ]; then
        cd "$UPDATE_SOURCE_DIR"
        
        local install_flags=""
        if [ "$CURRENT_INSTALL_TYPE" = "system" ]; then
            if [ "$EUID" -eq 0 ]; then
                install_flags="--system"
            else
                error "System installation requires root privileges. Run with sudo."
            fi
        else
            install_flags="--local"
        fi
        
        if [ "$FORCE_UPDATE" = true ]; then
            install_flags="$install_flags --force"
        fi
        
        if ! ./install.sh $install_flags; then
            error "Installation from package failed"
        fi
    else
        # Manual installation
        install_binaries_from_package
    fi
    
    log "Package installation completed"
}

# Install binaries from package
install_binaries_from_package() {
    log "Installing binaries manually..."
    
    # Install CLI tools
    if [ -f "$UPDATE_SOURCE_DIR/bin/chronicle" ]; then
        cp "$UPDATE_SOURCE_DIR/bin/chronicle" "$CURRENT_BIN_DIR/"
        chmod +x "$CURRENT_BIN_DIR/chronicle"
        info "Updated: $CURRENT_BIN_DIR/chronicle"
    fi
    
    if [ -f "$UPDATE_SOURCE_DIR/bin/chronicle-packer" ]; then
        cp "$UPDATE_SOURCE_DIR/bin/chronicle-packer" "$CURRENT_BIN_DIR/"
        chmod +x "$CURRENT_BIN_DIR/chronicle-packer"
        info "Updated: $CURRENT_BIN_DIR/chronicle-packer"
    fi
    
    # Install GUI app if available
    if [ -d "$UPDATE_SOURCE_DIR/Chronicle.app" ]; then
        local app_dest=""
        if [ "$CURRENT_INSTALL_TYPE" = "system" ]; then
            app_dest="/Applications/Chronicle.app"
        else
            app_dest="$HOME/Applications/Chronicle.app"
        fi
        
        if [ -d "$app_dest" ]; then
            rm -rf "$app_dest"
        fi
        
        cp -R "$UPDATE_SOURCE_DIR/Chronicle.app" "$app_dest"
        info "Updated: $app_dest"
    fi
}

# Install from source
install_from_source() {
    log "Installing from source..."
    
    # Install CLI binaries
    if [ -f "$UPDATE_SOURCE_DIR/cli/target/release/chronicle" ]; then
        cp "$UPDATE_SOURCE_DIR/cli/target/release/chronicle" "$CURRENT_BIN_DIR/"
        chmod +x "$CURRENT_BIN_DIR/chronicle"
        info "Updated: $CURRENT_BIN_DIR/chronicle"
    fi
    
    if [ -f "$UPDATE_SOURCE_DIR/packer/target/release/chronicle-packer" ]; then
        cp "$UPDATE_SOURCE_DIR/packer/target/release/chronicle-packer" "$CURRENT_BIN_DIR/"
        chmod +x "$CURRENT_BIN_DIR/chronicle-packer"
        info "Updated: $CURRENT_BIN_DIR/chronicle-packer"
    fi
    
    # Install GUI app if available
    local app_source=""
    if [ -d "$UPDATE_SOURCE_DIR/build/release/xcode/Build/Products/Release/ChronicleUI.app" ]; then
        app_source="$UPDATE_SOURCE_DIR/build/release/xcode/Build/Products/Release/ChronicleUI.app"
    elif [ -d "$UPDATE_SOURCE_DIR/ui/build/Release/ChronicleUI.app" ]; then
        app_source="$UPDATE_SOURCE_DIR/ui/build/Release/ChronicleUI.app"
    fi
    
    if [ -n "$app_source" ]; then
        local app_dest=""
        if [ "$CURRENT_INSTALL_TYPE" = "system" ]; then
            app_dest="/Applications/Chronicle.app"
        else
            app_dest="$HOME/Applications/Chronicle.app"
        fi
        
        if [ -d "$app_dest" ]; then
            rm -rf "$app_dest"
        fi
        
        cp -R "$app_source" "$app_dest"
        info "Updated: $app_dest"
    fi
    
    log "Source installation completed"
}

# Start Chronicle services
start_chronicle_services() {
    if [ "$AUTO_RESTART" = false ]; then
        return 0
    fi
    
    log "Starting Chronicle services..."
    
    # Start daemon if it exists
    if [ -f "/Library/LaunchDaemons/com.chronicle.daemon.plist" ]; then
        log "Starting system daemon..."
        launchctl start com.chronicle.daemon 2>/dev/null || true
    fi
    
    log "Chronicle services started"
}

# Verify update
verify_update() {
    log "Verifying update..."
    
    # Check if new version is installed
    if [ -f "$CURRENT_BIN_DIR/chronicle" ]; then
        local new_version=$("$CURRENT_BIN_DIR/chronicle" --version 2>/dev/null | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' || echo "unknown")
        info "Updated version: $new_version"
        
        if [ "$new_version" != "$CURRENT_VERSION" ] || [ "$FORCE_UPDATE" = true ]; then
            log "Update verification successful"
        else
            warn "Version did not change after update"
        fi
    else
        error "Chronicle binary not found after update"
    fi
}

# Cleanup update files
cleanup_update() {
    log "Cleaning up update files..."
    
    # Remove temporary directories
    if [[ "$UPDATE_SOURCE_DIR" =~ ^/tmp/ ]]; then
        rm -rf "$(dirname "$UPDATE_SOURCE_DIR")"
    fi
    
    log "Cleanup completed"
}

# Generate update report
generate_update_report() {
    log "Generating update report..."
    
    local report_file="$CURRENT_DATA_DIR/update_report.txt"
    mkdir -p "$(dirname "$report_file")"
    
    cat > "$report_file" << EOF
Chronicle Update Report
Generated: $(date)

Update Details:
  Source: $UPDATE_SOURCE
  Previous Version: $CURRENT_VERSION
  Updated Version: $LATEST_VERSION
  Installation Type: $CURRENT_INSTALL_TYPE

Configuration:
  Backup Created: $BACKUP_CONFIG
  Auto Restart: $AUTO_RESTART
  Force Update: $FORCE_UPDATE

Update Actions:
  - Stopped running services
  - Backed up current installation
  - Downloaded/prepared update
  - Installed new version
  - Verified installation
  - Started services (if requested)

EOF
    
    if [ "$BACKUP_CONFIG" = true ] && [ -f "/tmp/chronicle_last_backup" ]; then
        local backup_path=$(cat /tmp/chronicle_last_backup)
        echo "Backup Location: $backup_path" >> "$report_file"
    fi
    
    log "Update report saved to: $report_file"
}

# Main update function
main() {
    log "Starting Chronicle update..."
    
    parse_args "$@"
    
    info "Update configuration:"
    info "  Source: $UPDATE_SOURCE"
    info "  Check Only: $CHECK_ONLY"
    info "  Force Update: $FORCE_UPDATE"
    info "  Backup Config: $BACKUP_CONFIG"
    info "  Auto Restart: $AUTO_RESTART"
    
    detect_current_installation
    check_for_updates
    
    if [ "$CHECK_ONLY" = true ]; then
        log "Update check completed"
        exit 0
    fi
    
    backup_current_installation
    stop_chronicle_services
    
    case $UPDATE_SOURCE in
        "github")
            download_github_update
            ;;
        "url")
            download_url_update
            ;;
        "local")
            prepare_local_update
            ;;
    esac
    
    install_update
    start_chronicle_services
    verify_update
    cleanup_update
    generate_update_report
    
    log "Chronicle update completed successfully!"
    
    echo
    info "Update Summary:"
    info "  Previous Version: $CURRENT_VERSION"
    info "  Updated Version: $LATEST_VERSION"
    info "  Installation Type: $CURRENT_INSTALL_TYPE"
    
    if [ "$BACKUP_CONFIG" = true ] && [ -f "/tmp/chronicle_last_backup" ]; then
        local backup_path=$(cat /tmp/chronicle_last_backup)
        info "  Backup Location: $backup_path"
    fi
    
    echo
    info "Next steps:"
    info "  1. Test Chronicle functionality"
    info "  2. Check configuration files"
    if [ "$AUTO_RESTART" = false ]; then
        info "  3. Restart Chronicle services if needed"
    fi
}

# Run main function
main "$@"