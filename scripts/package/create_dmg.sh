#!/bin/bash

# Chronicle DMG Creation Script
# Creates a macOS DMG installer for Chronicle

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
APP_NAME="Chronicle"
VERSION=""
BUILD_DIR="$ROOT_DIR/build/release"
OUTPUT_DIR="$ROOT_DIR/dist/dmg"
DMG_NAME=""
VOLUME_NAME=""
BACKGROUND_IMAGE=""
WINDOW_SIZE="600,400"
ICON_SIZE=64
SIGN_DMG=false

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

Create a DMG installer for Chronicle.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    --version VERSION       Application version (required)
    --app-name NAME         Application name (default: Chronicle)
    --build-dir DIR         Build directory (default: build/release)
    --output-dir DIR        Output directory (default: dist/dmg)
    --dmg-name NAME         DMG filename (default: Chronicle-VERSION.dmg)
    --volume-name NAME      Volume name (default: Chronicle VERSION)
    --background IMAGE      Background image path
    --window-size SIZE      Window size (default: 600,400)
    --icon-size SIZE        Icon size (default: 64)
    --sign                  Sign the DMG

EXAMPLES:
    $0 --version 1.0.0
    $0 --version 1.0.0 --background assets/dmg-background.png
    $0 --version 1.0.0 --sign

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
            --app-name)
                APP_NAME="$2"
                shift 2
                ;;
            --build-dir)
                BUILD_DIR="$2"
                shift 2
                ;;
            --output-dir)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            --dmg-name)
                DMG_NAME="$2"
                shift 2
                ;;
            --volume-name)
                VOLUME_NAME="$2"
                shift 2
                ;;
            --background)
                BACKGROUND_IMAGE="$2"
                shift 2
                ;;
            --window-size)
                WINDOW_SIZE="$2"
                shift 2
                ;;
            --icon-size)
                ICON_SIZE="$2"
                shift 2
                ;;
            --sign)
                SIGN_DMG=true
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
    
    # Set defaults based on version
    if [ -z "$DMG_NAME" ]; then
        DMG_NAME="$APP_NAME-$VERSION.dmg"
    fi
    
    if [ -z "$VOLUME_NAME" ]; then
        VOLUME_NAME="$APP_NAME $VERSION"
    fi
    
    # Make paths absolute
    if [[ ! "$BUILD_DIR" = /* ]]; then
        BUILD_DIR="$ROOT_DIR/$BUILD_DIR"
    fi
    
    if [[ ! "$OUTPUT_DIR" = /* ]]; then
        OUTPUT_DIR="$ROOT_DIR/$OUTPUT_DIR"
    fi
}

# Check DMG creation prerequisites
check_dmg_prerequisites() {
    log "Checking DMG creation prerequisites..."
    
    # Check for macOS
    if [[ "$OSTYPE" != "darwin"* ]]; then
        error "DMG creation is only supported on macOS"
    fi
    
    # Check for required tools
    local required_tools=("hdiutil" "create-dmg")
    
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            if [ "$tool" = "create-dmg" ]; then
                warn "create-dmg not found. Install with: brew install create-dmg"
                warn "Falling back to manual DMG creation"
            else
                error "Required tool not found: $tool"
            fi
        fi
    done
    
    # Check for build artifacts
    if [ ! -d "$BUILD_DIR" ]; then
        error "Build directory not found: $BUILD_DIR"
    fi
    
    # Check for app bundle
    local app_bundle=""
    if [ -d "$BUILD_DIR/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/ChronicleUI.app"
    elif [ -d "$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app"
    else
        error "App bundle not found in build directory"
    fi
    
    info "Found app bundle: $app_bundle"
    
    log "Prerequisites check completed"
}

# Prepare DMG contents
prepare_dmg_contents() {
    log "Preparing DMG contents..."
    
    local dmg_staging="$OUTPUT_DIR/staging"
    
    # Clean and create staging directory
    rm -rf "$dmg_staging"
    mkdir -p "$dmg_staging"
    
    # Copy app bundle
    local app_bundle=""
    if [ -d "$BUILD_DIR/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/ChronicleUI.app"
    elif [ -d "$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app"
    fi
    
    if [ -n "$app_bundle" ]; then
        log "Copying app bundle..."
        cp -R "$app_bundle" "$dmg_staging/$APP_NAME.app"
    fi
    
    # Copy CLI tools
    local cli_tools_dir="$dmg_staging/Command Line Tools"
    mkdir -p "$cli_tools_dir"
    
    if [ -f "$BUILD_DIR/cli/chronicle" ]; then
        cp "$BUILD_DIR/cli/chronicle" "$cli_tools_dir/"
    fi
    
    if [ -f "$BUILD_DIR/packer/chronicle-packer" ]; then
        cp "$BUILD_DIR/packer/chronicle-packer" "$cli_tools_dir/"
    fi
    
    # Copy documentation
    local docs_dir="$dmg_staging/Documentation"
    mkdir -p "$docs_dir"
    
    if [ -f "$ROOT_DIR/README.md" ]; then
        cp "$ROOT_DIR/README.md" "$docs_dir/"
    fi
    
    if [ -f "$ROOT_DIR/LICENSE" ]; then
        cp "$ROOT_DIR/LICENSE" "$docs_dir/"
    fi
    
    # Create installation guide
    cat > "$docs_dir/Installation Guide.txt" << EOF
Chronicle Installation Guide

To install Chronicle:

1. Drag the Chronicle.app to your Applications folder
2. The Command Line Tools can be installed by running:
   sudo cp "Command Line Tools/"* /usr/local/bin/

For more information, see README.md
EOF
    
    # Create Applications symlink
    ln -sf /Applications "$dmg_staging/Applications"
    
    # Copy background image if provided
    if [ -n "$BACKGROUND_IMAGE" ] && [ -f "$BACKGROUND_IMAGE" ]; then
        cp "$BACKGROUND_IMAGE" "$dmg_staging/.background.png"
    fi
    
    log "DMG contents prepared in: $dmg_staging"
    echo "$dmg_staging"
}

# Create DMG with create-dmg tool
create_dmg_with_tool() {
    local dmg_staging="$1"
    local dmg_path="$OUTPUT_DIR/$DMG_NAME"
    
    log "Creating DMG with create-dmg tool..."
    
    # Remove existing DMG
    rm -f "$dmg_path"
    
    local create_dmg_flags=""
    create_dmg_flags="--volname \"$VOLUME_NAME\""
    create_dmg_flags="$create_dmg_flags --window-pos 200 120"
    create_dmg_flags="$create_dmg_flags --window-size ${WINDOW_SIZE/,/ }"
    create_dmg_flags="$create_dmg_flags --icon-size $ICON_SIZE"
    create_dmg_flags="$create_dmg_flags --icon \"$APP_NAME.app\" 150 150"
    create_dmg_flags="$create_dmg_flags --icon \"Applications\" 450 150"
    
    if [ -f "$dmg_staging/.background.png" ]; then
        create_dmg_flags="$create_dmg_flags --background \".background.png\""
    fi
    
    create_dmg_flags="$create_dmg_flags --app-drop-link 450 150"
    
    if [ "$VERBOSE" = true ]; then
        create_dmg_flags="$create_dmg_flags --verbose"
    fi
    
    # Create DMG
    eval "create-dmg $create_dmg_flags \"$dmg_path\" \"$dmg_staging\""
    
    if [ -f "$dmg_path" ]; then
        log "DMG created successfully: $dmg_path"
    else
        error "DMG creation failed"
    fi
}

# Create DMG manually with hdiutil
create_dmg_manually() {
    local dmg_staging="$1"
    local dmg_path="$OUTPUT_DIR/$DMG_NAME"
    
    log "Creating DMG manually with hdiutil..."
    
    # Remove existing DMG
    rm -f "$dmg_path"
    
    # Calculate size needed
    local size_kb=$(du -sk "$dmg_staging" | cut -f1)
    local size_mb=$((size_kb / 1024 + 50))  # Add 50MB padding
    
    # Create temporary DMG
    local temp_dmg="$OUTPUT_DIR/temp.dmg"
    rm -f "$temp_dmg"
    
    hdiutil create -srcfolder "$dmg_staging" \
                   -volname "$VOLUME_NAME" \
                   -fs HFS+ \
                   -fsargs "-c c=64,a=16,e=16" \
                   -format UDRW \
                   -size "${size_mb}m" \
                   "$temp_dmg"
    
    # Mount the DMG
    local mount_point="/Volumes/$VOLUME_NAME"
    hdiutil attach "$temp_dmg" -readwrite -noverify -noautoopen
    
    # Configure DMG appearance
    log "Configuring DMG appearance..."
    
    osascript << EOF
tell application "Finder"
    tell disk "$VOLUME_NAME"
        open
        set current view of container window to icon view
        set toolbar visible of container window to false
        set statusbar visible of container window to false
        set the bounds of container window to {400, 100, $(echo $WINDOW_SIZE | cut -d, -f1 | awk '{print $1+400}'), $(echo $WINDOW_SIZE | cut -d, -f2 | awk '{print $1+100}')}
        set arrangement of icon view options of container window to not arranged
        set icon size of icon view options of container window to $ICON_SIZE
        
        -- Position icons
        set position of item "$APP_NAME.app" of container window to {150, 150}
        set position of item "Applications" of container window to {450, 150}
        
        -- Set background if available
        if exists file ".background.png" then
            set background picture of icon view options of container window to file ".background.png"
        end if
        
        close
        open
        update without registering applications
        delay 2
    end tell
end tell
EOF
    
    # Unmount
    hdiutil detach "$mount_point"
    
    # Convert to compressed DMG
    hdiutil convert "$temp_dmg" \
                    -format UDZO \
                    -imagekey zlib-level=9 \
                    -o "$dmg_path"
    
    # Clean up
    rm -f "$temp_dmg"
    
    if [ -f "$dmg_path" ]; then
        log "DMG created successfully: $dmg_path"
    else
        error "DMG creation failed"
    fi
}

# Sign DMG
sign_dmg() {
    local dmg_path="$OUTPUT_DIR/$DMG_NAME"
    
    if [ "$SIGN_DMG" = false ]; then
        return 0
    fi
    
    log "Signing DMG..."
    
    # Check for signing certificate
    local cert_name="Developer ID Application:"
    if ! security find-identity -v -p codesigning | grep -q "$cert_name"; then
        warn "Code signing certificate not found, skipping DMG signing"
        return 0
    fi
    
    if ! codesign --sign "$cert_name" --timestamp "$dmg_path"; then
        warn "DMG signing failed"
        return 1
    fi
    
    # Verify signature
    if ! codesign --verify --strict "$dmg_path"; then
        warn "DMG signature verification failed"
        return 1
    fi
    
    log "DMG signed successfully"
}

# Validate DMG
validate_dmg() {
    local dmg_path="$OUTPUT_DIR/$DMG_NAME"
    
    log "Validating DMG..."
    
    # Check if DMG exists
    if [ ! -f "$dmg_path" ]; then
        error "DMG not found: $dmg_path"
    fi
    
    # Verify DMG
    if ! hdiutil verify "$dmg_path"; then
        error "DMG verification failed"
    fi
    
    # Test mount
    local test_mount="/tmp/chronicle_dmg_test"
    mkdir -p "$test_mount"
    
    if hdiutil attach "$dmg_path" -mountpoint "$test_mount" -readonly -nobrowse; then
        # Check contents
        if [ -d "$test_mount/$APP_NAME.app" ]; then
            info "App bundle found in DMG"
        else
            warn "App bundle not found in DMG"
        fi
        
        if [ -d "$test_mount/Command Line Tools" ]; then
            info "Command Line Tools found in DMG"
        fi
        
        # Unmount
        hdiutil detach "$test_mount"
    else
        error "Failed to mount DMG for validation"
    fi
    
    rmdir "$test_mount" 2>/dev/null || true
    
    log "DMG validation completed"
}

# Generate DMG information
generate_dmg_info() {
    local dmg_path="$OUTPUT_DIR/$DMG_NAME"
    local info_file="$OUTPUT_DIR/$APP_NAME-$VERSION-dmg-info.txt"
    
    log "Generating DMG information..."
    
    cat > "$info_file" << EOF
Chronicle DMG Information
Generated: $(date)

DMG Details:
  Name: $DMG_NAME
  Path: $dmg_path
  Version: $VERSION
  Volume Name: $VOLUME_NAME
  
File Information:
  Size: $(ls -lh "$dmg_path" | awk '{print $5}')
  SHA256: $(shasum -a 256 "$dmg_path" | awk '{print $1}')
  
Contents:
  - $APP_NAME.app (Main application)
  - Command Line Tools (CLI utilities)
  - Documentation (README, LICENSE, Installation Guide)
  - Applications symlink (for easy installation)

Installation:
  1. Open the DMG file
  2. Drag $APP_NAME.app to Applications folder
  3. Optionally install Command Line Tools

EOF
    
    # Add signing information if signed
    if [ "$SIGN_DMG" = true ]; then
        echo "Code Signing:" >> "$info_file"
        codesign -dv "$dmg_path" 2>&1 | grep -E "(Authority|Identifier|TeamIdentifier)" >> "$info_file" || true
    fi
    
    log "DMG information saved to: $info_file"
}

# Cleanup staging directory
cleanup_staging() {
    local dmg_staging="$OUTPUT_DIR/staging"
    
    if [ -d "$dmg_staging" ]; then
        log "Cleaning up staging directory..."
        rm -rf "$dmg_staging"
    fi
}

# Main DMG creation function
main() {
    log "Starting Chronicle DMG creation..."
    
    parse_args "$@"
    
    info "DMG creation configuration:"
    info "  App Name: $APP_NAME"
    info "  Version: $VERSION"
    info "  Build Directory: $BUILD_DIR"
    info "  Output Directory: $OUTPUT_DIR"
    info "  DMG Name: $DMG_NAME"
    info "  Volume Name: $VOLUME_NAME"
    info "  Sign DMG: $SIGN_DMG"
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    check_dmg_prerequisites
    
    local dmg_staging
    dmg_staging=$(prepare_dmg_contents)
    
    # Try to use create-dmg first, fall back to manual creation
    if command -v create-dmg &> /dev/null; then
        create_dmg_with_tool "$dmg_staging"
    else
        create_dmg_manually "$dmg_staging"
    fi
    
    sign_dmg
    validate_dmg
    generate_dmg_info
    cleanup_staging
    
    log "DMG creation completed successfully!"
    info "DMG file: $OUTPUT_DIR/$DMG_NAME"
    
    # Open output directory
    if command -v open &> /dev/null; then
        open "$OUTPUT_DIR"
    fi
}

# Run main function
main "$@"