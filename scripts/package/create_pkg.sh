#!/bin/bash

# Chronicle PKG Creation Script
# Creates a macOS PKG installer for Chronicle

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
OUTPUT_DIR="$ROOT_DIR/dist/pkg"
PKG_NAME=""
BUNDLE_ID="com.chronicle.installer"
SIGN_PKG=false
NOTARIZE_PKG=false

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

Create a PKG installer for Chronicle.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    --version VERSION       Application version (required)
    --app-name NAME         Application name (default: Chronicle)
    --build-dir DIR         Build directory (default: build/release)
    --output-dir DIR        Output directory (default: dist/pkg)
    --pkg-name NAME         PKG filename (default: Chronicle-VERSION.pkg)
    --bundle-id ID          Bundle identifier (default: com.chronicle.installer)
    --sign                  Sign the PKG
    --notarize              Notarize the PKG (requires signing)

EXAMPLES:
    $0 --version 1.0.0
    $0 --version 1.0.0 --sign
    $0 --version 1.0.0 --sign --notarize

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
            --pkg-name)
                PKG_NAME="$2"
                shift 2
                ;;
            --bundle-id)
                BUNDLE_ID="$2"
                shift 2
                ;;
            --sign)
                SIGN_PKG=true
                shift
                ;;
            --notarize)
                NOTARIZE_PKG=true
                SIGN_PKG=true  # Notarization requires signing
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
    if [ -z "$PKG_NAME" ]; then
        PKG_NAME="$APP_NAME-$VERSION.pkg"
    fi
    
    # Make paths absolute
    if [[ ! "$BUILD_DIR" = /* ]]; then
        BUILD_DIR="$ROOT_DIR/$BUILD_DIR"
    fi
    
    if [[ ! "$OUTPUT_DIR" = /* ]]; then
        OUTPUT_DIR="$ROOT_DIR/$OUTPUT_DIR"
    fi
}

# Check PKG creation prerequisites
check_pkg_prerequisites() {
    log "Checking PKG creation prerequisites..."
    
    # Check for macOS
    if [[ "$OSTYPE" != "darwin"* ]]; then
        error "PKG creation is only supported on macOS"
    fi
    
    # Check for required tools
    local required_tools=("pkgbuild" "productbuild")
    
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            error "Required tool not found: $tool"
        fi
    done
    
    # Check for build artifacts
    if [ ! -d "$BUILD_DIR" ]; then
        error "Build directory not found: $BUILD_DIR"
    fi
    
    log "Prerequisites check completed"
}

# Prepare PKG payload
prepare_pkg_payload() {
    log "Preparing PKG payload..."
    
    local pkg_staging="$OUTPUT_DIR/staging"
    local pkg_root="$pkg_staging/root"
    local pkg_scripts="$pkg_staging/scripts"
    
    # Clean and create staging directories
    rm -rf "$pkg_staging"
    mkdir -p "$pkg_root"
    mkdir -p "$pkg_scripts"
    
    # Create installation structure
    mkdir -p "$pkg_root/Applications"
    mkdir -p "$pkg_root/usr/local/bin"
    mkdir -p "$pkg_root/usr/local/share/chronicle"
    mkdir -p "$pkg_root/Library/LaunchDaemons"
    
    # Copy app bundle
    local app_bundle=""
    if [ -d "$BUILD_DIR/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/ChronicleUI.app"
    elif [ -d "$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app"
    fi
    
    if [ -n "$app_bundle" ]; then
        log "Copying app bundle..."
        cp -R "$app_bundle" "$pkg_root/Applications/$APP_NAME.app"
    fi
    
    # Copy CLI tools
    if [ -f "$BUILD_DIR/cli/chronicle" ]; then
        cp "$BUILD_DIR/cli/chronicle" "$pkg_root/usr/local/bin/"
        chmod +x "$pkg_root/usr/local/bin/chronicle"
    fi
    
    if [ -f "$BUILD_DIR/packer/chronicle-packer" ]; then
        cp "$BUILD_DIR/packer/chronicle-packer" "$pkg_root/usr/local/bin/"
        chmod +x "$pkg_root/usr/local/bin/chronicle-packer"
    fi
    
    # Copy collectors framework
    if [ -d "$BUILD_DIR/xcode/Build/Products/Release/ChronicleCollectors.framework" ]; then
        mkdir -p "$pkg_root/usr/local/lib"
        cp -R "$BUILD_DIR/xcode/Build/Products/Release/ChronicleCollectors.framework" "$pkg_root/usr/local/lib/"
    fi
    
    # Copy configuration files
    if [ -f "$ROOT_DIR/config/chronicle.toml.example" ]; then
        cp "$ROOT_DIR/config/chronicle.toml.example" "$pkg_root/usr/local/share/chronicle/chronicle.toml.example"
    fi
    
    # Copy documentation
    if [ -f "$ROOT_DIR/README.md" ]; then
        cp "$ROOT_DIR/README.md" "$pkg_root/usr/local/share/chronicle/"
    fi
    
    if [ -f "$ROOT_DIR/LICENSE" ]; then
        cp "$ROOT_DIR/LICENSE" "$pkg_root/usr/local/share/chronicle/"
    fi
    
    # Create launch daemon plist
    create_launch_daemon "$pkg_root/Library/LaunchDaemons"
    
    # Create installation scripts
    create_installation_scripts "$pkg_scripts"
    
    log "PKG payload prepared in: $pkg_staging"
    echo "$pkg_staging"
}

# Create launch daemon plist
create_launch_daemon() {
    local daemon_dir="$1"
    local plist_file="$daemon_dir/com.chronicle.daemon.plist"
    
    log "Creating launch daemon plist..."
    
    cat > "$plist_file" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.chronicle.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/chronicle</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <false/>
    <key>KeepAlive</key>
    <false/>
    <key>StandardOutPath</key>
    <string>/var/log/chronicle.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/chronicle.error.log</string>
</dict>
</plist>
EOF
}

# Create installation scripts
create_installation_scripts() {
    local scripts_dir="$1"
    
    log "Creating installation scripts..."
    
    # Pre-installation script
    cat > "$scripts_dir/preinstall" << 'EOF'
#!/bin/bash
# Chronicle pre-installation script

set -e

echo "Preparing for Chronicle installation..."

# Stop any running Chronicle processes
pkill -f chronicle || true

# Remove old versions if they exist
if [ -d "/Applications/Chronicle.app" ]; then
    echo "Removing existing Chronicle application..."
    rm -rf "/Applications/Chronicle.app"
fi

# Unload launch daemon if it exists
if [ -f "/Library/LaunchDaemons/com.chronicle.daemon.plist" ]; then
    launchctl unload "/Library/LaunchDaemons/com.chronicle.daemon.plist" 2>/dev/null || true
fi

echo "Pre-installation completed"
exit 0
EOF
    
    # Post-installation script
    cat > "$scripts_dir/postinstall" << 'EOF'
#!/bin/bash
# Chronicle post-installation script

set -e

echo "Completing Chronicle installation..."

# Set proper permissions
chown -R root:wheel /usr/local/bin/chronicle* 2>/dev/null || true
chown -R root:wheel /usr/local/lib/ChronicleCollectors.framework 2>/dev/null || true
chown -R root:wheel /usr/local/share/chronicle 2>/dev/null || true

# Set executable permissions
chmod +x /usr/local/bin/chronicle 2>/dev/null || true
chmod +x /usr/local/bin/chronicle-packer 2>/dev/null || true

# Create configuration directory
mkdir -p /usr/local/etc/chronicle
if [ ! -f "/usr/local/etc/chronicle/chronicle.toml" ] && [ -f "/usr/local/share/chronicle/chronicle.toml.example" ]; then
    cp /usr/local/share/chronicle/chronicle.toml.example /usr/local/etc/chronicle/chronicle.toml
fi

# Load launch daemon (but don't start it automatically)
if [ -f "/Library/LaunchDaemons/com.chronicle.daemon.plist" ]; then
    launchctl load -w "/Library/LaunchDaemons/com.chronicle.daemon.plist" 2>/dev/null || true
fi

# Update PATH in common shell profiles
for profile in /etc/profile /etc/bash_profile /etc/zsh_profile; do
    if [ -f "$profile" ]; then
        if ! grep -q "/usr/local/bin" "$profile"; then
            echo 'export PATH="/usr/local/bin:$PATH"' >> "$profile"
        fi
    fi
done

echo "Post-installation completed"
echo "Chronicle has been installed successfully!"
echo "You can now use 'chronicle' command in Terminal"
exit 0
EOF
    
    # Pre-removal script
    cat > "$scripts_dir/preremove" << 'EOF'
#!/bin/bash
# Chronicle pre-removal script

set -e

echo "Preparing for Chronicle removal..."

# Stop and unload launch daemon
if [ -f "/Library/LaunchDaemons/com.chronicle.daemon.plist" ]; then
    launchctl unload "/Library/LaunchDaemons/com.chronicle.daemon.plist" 2>/dev/null || true
fi

# Stop any running Chronicle processes
pkill -f chronicle || true

echo "Pre-removal completed"
exit 0
EOF
    
    # Make scripts executable
    chmod +x "$scripts_dir"/*
}

# Create distribution file
create_distribution_file() {
    local pkg_staging="$1"
    local dist_file="$pkg_staging/distribution.xml"
    
    log "Creating distribution file..."
    
    cat > "$dist_file" << EOF
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
    <title>$APP_NAME $VERSION</title>
    <organization>$BUNDLE_ID</organization>
    <domains enable_localSystem="true"/>
    <options customize="never" require-scripts="false" rootVolumeOnly="true"/>
    
    <!-- Background -->
    <background file="background.png" mime-type="image/png" alignment="center" scaling="tofit"/>
    
    <!-- Welcome -->
    <welcome file="welcome.html" mime-type="text/html"/>
    
    <!-- License -->
    <license file="license.txt" mime-type="text/plain"/>
    
    <!-- Conclusion -->
    <conclusion file="conclusion.html" mime-type="text/html"/>
    
    <pkg-ref id="$BUNDLE_ID.pkg"/>
    
    <choices-outline>
        <line choice="default">
            <line choice="$BUNDLE_ID.pkg"/>
        </line>
    </choices-outline>
    
    <choice id="default"/>
    <choice id="$BUNDLE_ID.pkg" visible="false">
        <pkg-ref id="$BUNDLE_ID.pkg"/>
    </choice>
    
    <pkg-ref id="$BUNDLE_ID.pkg" version="$VERSION" onConclusion="none">chronicle.pkg</pkg-ref>
</installer-gui-script>
EOF
}

# Create installer resources
create_installer_resources() {
    local pkg_staging="$1"
    local resources_dir="$pkg_staging/resources"
    
    log "Creating installer resources..."
    
    mkdir -p "$resources_dir"
    
    # Welcome page
    cat > "$resources_dir/welcome.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Welcome to Chronicle</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
        h1 { color: #333; }
        p { line-height: 1.6; }
    </style>
</head>
<body>
    <h1>Welcome to Chronicle $VERSION</h1>
    <p>This installer will install Chronicle on your Mac.</p>
    <p>Chronicle is a comprehensive activity monitoring and data management system that provides:</p>
    <ul>
        <li>Real-time activity monitoring</li>
        <li>Efficient data storage and compression</li>
        <li>Powerful search and query capabilities</li>
        <li>Command-line tools for automation</li>
    </ul>
    <p>Click Continue to proceed with the installation.</p>
</body>
</html>
EOF
    
    # Conclusion page
    cat > "$resources_dir/conclusion.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Installation Complete</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
        h1 { color: #333; }
        p { line-height: 1.6; }
    </style>
</head>
<body>
    <h1>Installation Complete</h1>
    <p>Chronicle has been successfully installed on your Mac.</p>
    
    <h2>What's Installed:</h2>
    <ul>
        <li><strong>Chronicle.app</strong> - Main application in /Applications</li>
        <li><strong>chronicle</strong> - Command-line tool in /usr/local/bin</li>
        <li><strong>chronicle-packer</strong> - Data processing tool in /usr/local/bin</li>
        <li><strong>Configuration files</strong> - In /usr/local/share/chronicle</li>
    </ul>
    
    <h2>Getting Started:</h2>
    <p>You can now:</p>
    <ul>
        <li>Launch Chronicle from Applications folder</li>
        <li>Use <code>chronicle</code> command in Terminal</li>
        <li>Configure settings in /usr/local/etc/chronicle/chronicle.toml</li>
    </ul>
    
    <p>For more information, see the documentation in /usr/local/share/chronicle/</p>
</body>
</html>
EOF
    
    # License file
    if [ -f "$ROOT_DIR/LICENSE" ]; then
        cp "$ROOT_DIR/LICENSE" "$resources_dir/license.txt"
    else
        cat > "$resources_dir/license.txt" << EOF
Chronicle Software License

This software is provided as-is without any warranty.
Please see the project documentation for full license terms.
EOF
    fi
    
    # Create a simple background image (placeholder)
    # In a real implementation, you'd want to provide an actual image
    touch "$resources_dir/background.png"
}

# Build component package
build_component_package() {
    local pkg_staging="$1"
    local component_pkg="$pkg_staging/chronicle.pkg"
    
    log "Building component package..."
    
    local pkgbuild_flags=""
    pkgbuild_flags="--root $pkg_staging/root"
    pkgbuild_flags="$pkgbuild_flags --scripts $pkg_staging/scripts"
    pkgbuild_flags="$pkgbuild_flags --identifier $BUNDLE_ID.pkg"
    pkgbuild_flags="$pkgbuild_flags --version $VERSION"
    pkgbuild_flags="$pkgbuild_flags --install-location /"
    
    if [ "$SIGN_PKG" = true ]; then
        local cert_name="Developer ID Installer:"
        if security find-identity -v -p codesigning | grep -q "$cert_name"; then
            pkgbuild_flags="$pkgbuild_flags --sign \"$cert_name\""
        else
            warn "Installer signing certificate not found"
        fi
    fi
    
    if [ "$VERBOSE" = true ]; then
        pkgbuild_flags="$pkgbuild_flags --verbose"
    fi
    
    eval "pkgbuild $pkgbuild_flags \"$component_pkg\""
    
    if [ ! -f "$component_pkg" ]; then
        error "Component package creation failed"
    fi
    
    log "Component package created: $component_pkg"
}

# Build product package
build_product_package() {
    local pkg_staging="$1"
    local final_pkg="$OUTPUT_DIR/$PKG_NAME"
    
    log "Building product package..."
    
    # Remove existing final package
    rm -f "$final_pkg"
    
    local productbuild_flags=""
    productbuild_flags="--distribution $pkg_staging/distribution.xml"
    productbuild_flags="$productbuild_flags --package-path $pkg_staging"
    productbuild_flags="$productbuild_flags --resources $pkg_staging/resources"
    
    if [ "$SIGN_PKG" = true ]; then
        local cert_name="Developer ID Installer:"
        if security find-identity -v -p codesigning | grep -q "$cert_name"; then
            productbuild_flags="$productbuild_flags --sign \"$cert_name\""
        else
            warn "Installer signing certificate not found"
        fi
    fi
    
    if [ "$VERBOSE" = true ]; then
        productbuild_flags="$productbuild_flags --verbose"
    fi
    
    eval "productbuild $productbuild_flags \"$final_pkg\""
    
    if [ ! -f "$final_pkg" ]; then
        error "Product package creation failed"
    fi
    
    log "Product package created: $final_pkg"
}

# Notarize PKG
notarize_pkg() {
    local pkg_path="$OUTPUT_DIR/$PKG_NAME"
    
    if [ "$NOTARIZE_PKG" = false ]; then
        return 0
    fi
    
    log "Notarizing PKG..."
    
    # Check for notarization credentials
    if [ -z "${NOTARIZE_USERNAME:-}" ] || [ -z "${NOTARIZE_PASSWORD:-}" ]; then
        warn "Notarization credentials not set. Set NOTARIZE_USERNAME and NOTARIZE_PASSWORD environment variables."
        return 0
    fi
    
    # Submit for notarization
    if ! xcrun altool --notarize-app \
                     --primary-bundle-id "$BUNDLE_ID" \
                     --username "$NOTARIZE_USERNAME" \
                     --password "$NOTARIZE_PASSWORD" \
                     --file "$pkg_path"; then
        warn "PKG notarization submission failed"
        return 1
    fi
    
    log "PKG submitted for notarization"
}

# Validate PKG
validate_pkg() {
    local pkg_path="$OUTPUT_DIR/$PKG_NAME"
    
    log "Validating PKG..."
    
    # Check if PKG exists
    if [ ! -f "$pkg_path" ]; then
        error "PKG not found: $pkg_path"
    fi
    
    # Verify PKG structure
    if ! pkgutil --check-signature "$pkg_path"; then
        warn "PKG signature check failed"
    fi
    
    # List PKG contents
    if [ "$VERBOSE" = true ]; then
        log "PKG contents:"
        pkgutil --payload-files "$pkg_path" | head -20
    fi
    
    log "PKG validation completed"
}

# Generate PKG information
generate_pkg_info() {
    local pkg_path="$OUTPUT_DIR/$PKG_NAME"
    local info_file="$OUTPUT_DIR/$APP_NAME-$VERSION-pkg-info.txt"
    
    log "Generating PKG information..."
    
    cat > "$info_file" << EOF
Chronicle PKG Information
Generated: $(date)

PKG Details:
  Name: $PKG_NAME
  Path: $pkg_path
  Version: $VERSION
  Bundle ID: $BUNDLE_ID
  
File Information:
  Size: $(ls -lh "$pkg_path" | awk '{print $5}')
  SHA256: $(shasum -a 256 "$pkg_path" | awk '{print $1}')
  
Installation:
  The PKG installer will install Chronicle system-wide:
  - Chronicle.app in /Applications
  - CLI tools in /usr/local/bin
  - Frameworks in /usr/local/lib
  - Configuration in /usr/local/etc/chronicle
  - Documentation in /usr/local/share/chronicle

Uninstallation:
  To uninstall, run: sudo chronicle uninstall

EOF
    
    # Add signing information if signed
    if [ "$SIGN_PKG" = true ]; then
        echo "Code Signing:" >> "$info_file"
        pkgutil --check-signature "$pkg_path" >> "$info_file" 2>&1 || true
    fi
    
    log "PKG information saved to: $info_file"
}

# Cleanup staging directory
cleanup_staging() {
    local pkg_staging="$OUTPUT_DIR/staging"
    
    if [ -d "$pkg_staging" ]; then
        log "Cleaning up staging directory..."
        rm -rf "$pkg_staging"
    fi
}

# Main PKG creation function
main() {
    log "Starting Chronicle PKG creation..."
    
    parse_args "$@"
    
    info "PKG creation configuration:"
    info "  App Name: $APP_NAME"
    info "  Version: $VERSION"
    info "  Build Directory: $BUILD_DIR"
    info "  Output Directory: $OUTPUT_DIR"
    info "  PKG Name: $PKG_NAME"
    info "  Bundle ID: $BUNDLE_ID"
    info "  Sign PKG: $SIGN_PKG"
    info "  Notarize PKG: $NOTARIZE_PKG"
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    check_pkg_prerequisites
    
    local pkg_staging
    pkg_staging=$(prepare_pkg_payload)
    
    create_distribution_file "$pkg_staging"
    create_installer_resources "$pkg_staging"
    build_component_package "$pkg_staging"
    build_product_package "$pkg_staging"
    notarize_pkg
    validate_pkg
    generate_pkg_info
    cleanup_staging
    
    log "PKG creation completed successfully!"
    info "PKG file: $OUTPUT_DIR/$PKG_NAME"
    
    # Open output directory
    if command -v open &> /dev/null; then
        open "$OUTPUT_DIR"
    fi
}

# Run main function
main "$@"