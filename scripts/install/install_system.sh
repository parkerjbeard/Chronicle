#!/bin/bash

# Chronicle System-wide Installation Script
# Installs Chronicle system-wide for all users

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VERBOSE=false
FORCE_INSTALL=false
SOURCE_DIR=""
INSTALL_APP=true
INSTALL_CLI=true
INSTALL_CONFIG=true
CREATE_DAEMON=true

# System paths
SYSTEM_BIN_DIR="/usr/local/bin"
SYSTEM_LIB_DIR="/usr/local/lib"
SYSTEM_SHARE_DIR="/usr/local/share/chronicle"
SYSTEM_CONFIG_DIR="/usr/local/etc/chronicle"
APPLICATIONS_DIR="/Applications"
DAEMON_PLIST="/Library/LaunchDaemons/com.chronicle.daemon.plist"

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

Install Chronicle system-wide for all users.
This script requires administrator privileges.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -f, --force             Force installation (overwrite existing)
    --source-dir DIR        Source directory (auto-detected if not specified)
    --no-app                Don't install GUI application
    --no-cli                Don't install CLI tools
    --no-config             Don't install configuration files
    --no-daemon             Don't create launch daemon
    --app-only              Install only GUI application
    --cli-only              Install only CLI tools

EXAMPLES:
    sudo $0                 # Full system installation
    sudo $0 --source-dir /path/to/chronicle
    sudo $0 --cli-only      # Install CLI tools only
    sudo $0 --force         # Force reinstallation

NOTE: This script must be run as root (use sudo).

EOF
}

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        error "This script must be run as root. Use: sudo $0"
    fi
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
                FORCE_INSTALL=true
                shift
                ;;
            --source-dir)
                SOURCE_DIR="$2"
                shift 2
                ;;
            --no-app)
                INSTALL_APP=false
                shift
                ;;
            --no-cli)
                INSTALL_CLI=false
                shift
                ;;
            --no-config)
                INSTALL_CONFIG=false
                shift
                ;;
            --no-daemon)
                CREATE_DAEMON=false
                shift
                ;;
            --app-only)
                INSTALL_APP=true
                INSTALL_CLI=false
                INSTALL_CONFIG=false
                CREATE_DAEMON=false
                shift
                ;;
            --cli-only)
                INSTALL_APP=false
                INSTALL_CLI=true
                INSTALL_CONFIG=true
                CREATE_DAEMON=true
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
    
    # Auto-detect source directory if not provided
    if [ -z "$SOURCE_DIR" ]; then
        local script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
        local potential_source="$(dirname "$(dirname "$script_dir")")"
        
        if [ -f "$potential_source/Cargo.toml" ] || [ -f "$potential_source/Chronicle.xcworkspace" ]; then
            SOURCE_DIR="$potential_source"
            info "Auto-detected source directory: $SOURCE_DIR"
        else
            error "Cannot auto-detect source directory. Use --source-dir to specify."
        fi
    fi
    
    # Make source directory absolute
    SOURCE_DIR="$(cd "$SOURCE_DIR" && pwd)"
}

# Check installation prerequisites
check_install_prerequisites() {
    log "Checking installation prerequisites..."
    
    # Check if source directory exists
    if [ ! -d "$SOURCE_DIR" ]; then
        error "Source directory not found: $SOURCE_DIR"
    fi
    
    # Check for macOS
    if [[ "$OSTYPE" != "darwin"* ]]; then
        error "This script is designed for macOS only"
    fi
    
    # Check for required source files
    local required_files=()
    
    if [ "$INSTALL_CLI" = true ]; then
        required_files+=("cli/target/release/chronicle" "packer/target/release/chronicle-packer")
    fi
    
    if [ "$INSTALL_APP" = true ]; then
        required_files+=("ui/ChronicleUI.app" "build/release/xcode/Build/Products/Release/ChronicleUI.app")
    fi
    
    # Check for at least one required file
    local found_files=0
    for file in "${required_files[@]}"; do
        if [ -e "$SOURCE_DIR/$file" ]; then
            ((found_files++))
        fi
    done
    
    if [ $found_files -eq 0 ]; then
        warn "No built binaries found. Please build Chronicle first."
        warn "Run: ./scripts/dev/build.sh --release"
        error "Required binaries not found"
    fi
    
    log "Prerequisites check completed"
}

# Check for existing installation
check_existing_installation() {
    log "Checking for existing installation..."
    
    local existing_files=()
    
    # Check for existing binaries
    if [ -f "$SYSTEM_BIN_DIR/chronicle" ]; then
        existing_files+=("$SYSTEM_BIN_DIR/chronicle")
    fi
    
    if [ -f "$SYSTEM_BIN_DIR/chronicle-packer" ]; then
        existing_files+=("$SYSTEM_BIN_DIR/chronicle-packer")
    fi
    
    # Check for existing app
    if [ -d "$APPLICATIONS_DIR/Chronicle.app" ]; then
        existing_files+=("$APPLICATIONS_DIR/Chronicle.app")
    fi
    
    # Check for existing daemon
    if [ -f "$DAEMON_PLIST" ]; then
        existing_files+=("$DAEMON_PLIST")
    fi
    
    # Check for existing config
    if [ -d "$SYSTEM_CONFIG_DIR" ]; then
        existing_files+=("$SYSTEM_CONFIG_DIR")
    fi
    
    if [ ${#existing_files[@]} -gt 0 ]; then
        warn "Existing Chronicle installation found:"
        for file in "${existing_files[@]}"; do
            warn "  $file"
        done
        
        if [ "$FORCE_INSTALL" = false ]; then
            echo
            read -p "Do you want to continue and overwrite existing files? [y/N] " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                info "Installation cancelled"
                exit 0
            fi
        fi
        
        # Stop and unload daemon if it exists
        if [ -f "$DAEMON_PLIST" ]; then
            log "Stopping existing daemon..."
            launchctl unload "$DAEMON_PLIST" 2>/dev/null || true
        fi
    fi
    
    log "Existing installation check completed"
}

# Create system directories
create_system_directories() {
    log "Creating system directories..."
    
    local directories=("$SYSTEM_BIN_DIR" "$SYSTEM_LIB_DIR" "$SYSTEM_SHARE_DIR" "$SYSTEM_CONFIG_DIR")
    
    for dir in "${directories[@]}"; do
        if [ ! -d "$dir" ]; then
            mkdir -p "$dir"
            if [ "$VERBOSE" = true ]; then
                info "Created directory: $dir"
            fi
        fi
    done
    
    log "System directories created"
}

# Install CLI tools
install_system_cli_tools() {
    if [ "$INSTALL_CLI" = false ]; then
        return 0
    fi
    
    log "Installing system CLI tools..."
    
    # Find and install chronicle binary
    local chronicle_bin=""
    if [ -f "$SOURCE_DIR/cli/target/release/chronicle" ]; then
        chronicle_bin="$SOURCE_DIR/cli/target/release/chronicle"
    elif [ -f "$SOURCE_DIR/build/release/cli/chronicle" ]; then
        chronicle_bin="$SOURCE_DIR/build/release/cli/chronicle"
    fi
    
    if [ -n "$chronicle_bin" ]; then
        cp "$chronicle_bin" "$SYSTEM_BIN_DIR/chronicle"
        chown root:wheel "$SYSTEM_BIN_DIR/chronicle"
        chmod 755 "$SYSTEM_BIN_DIR/chronicle"
        info "Installed chronicle CLI to $SYSTEM_BIN_DIR/chronicle"
    else
        warn "Chronicle CLI binary not found"
    fi
    
    # Find and install packer binary
    local packer_bin=""
    if [ -f "$SOURCE_DIR/packer/target/release/chronicle-packer" ]; then
        packer_bin="$SOURCE_DIR/packer/target/release/chronicle-packer"
    elif [ -f "$SOURCE_DIR/build/release/packer/chronicle-packer" ]; then
        packer_bin="$SOURCE_DIR/build/release/packer/chronicle-packer"
    fi
    
    if [ -n "$packer_bin" ]; then
        cp "$packer_bin" "$SYSTEM_BIN_DIR/chronicle-packer"
        chown root:wheel "$SYSTEM_BIN_DIR/chronicle-packer"
        chmod 755 "$SYSTEM_BIN_DIR/chronicle-packer"
        info "Installed chronicle-packer to $SYSTEM_BIN_DIR/chronicle-packer"
    else
        warn "Chronicle packer binary not found"
    fi
    
    # Install ring buffer library if available
    if [ -f "$SOURCE_DIR/ring-buffer/libringbuffer.a" ]; then
        cp "$SOURCE_DIR/ring-buffer/libringbuffer.a" "$SYSTEM_LIB_DIR/"
        chown root:wheel "$SYSTEM_LIB_DIR/libringbuffer.a"
        chmod 644 "$SYSTEM_LIB_DIR/libringbuffer.a"
        info "Installed ring buffer library to $SYSTEM_LIB_DIR/libringbuffer.a"
    fi
    
    # Install collectors framework if available
    if [ -d "$SOURCE_DIR/build/release/xcode/Build/Products/Release/ChronicleCollectors.framework" ]; then
        cp -R "$SOURCE_DIR/build/release/xcode/Build/Products/Release/ChronicleCollectors.framework" "$SYSTEM_LIB_DIR/"
        chown -R root:wheel "$SYSTEM_LIB_DIR/ChronicleCollectors.framework"
        info "Installed collectors framework to $SYSTEM_LIB_DIR/ChronicleCollectors.framework"
    fi
    
    log "System CLI tools installation completed"
}

# Install GUI application
install_system_gui_app() {
    if [ "$INSTALL_APP" = false ]; then
        return 0
    fi
    
    log "Installing system GUI application..."
    
    # Find app bundle
    local app_bundle=""
    if [ -d "$SOURCE_DIR/build/release/xcode/Build/Products/Release/ChronicleUI.app" ]; then
        app_bundle="$SOURCE_DIR/build/release/xcode/Build/Products/Release/ChronicleUI.app"
    elif [ -d "$SOURCE_DIR/ui/build/Release/ChronicleUI.app" ]; then
        app_bundle="$SOURCE_DIR/ui/build/Release/ChronicleUI.app"
    elif [ -d "$SOURCE_DIR/ChronicleUI.app" ]; then
        app_bundle="$SOURCE_DIR/ChronicleUI.app"
    fi
    
    if [ -n "$app_bundle" ]; then
        # Remove existing app
        if [ -d "$APPLICATIONS_DIR/Chronicle.app" ]; then
            rm -rf "$APPLICATIONS_DIR/Chronicle.app"
        fi
        
        # Copy app bundle
        cp -R "$app_bundle" "$APPLICATIONS_DIR/Chronicle.app"
        chown -R root:admin "$APPLICATIONS_DIR/Chronicle.app"
        chmod -R 755 "$APPLICATIONS_DIR/Chronicle.app"
        info "Installed Chronicle.app to $APPLICATIONS_DIR/Chronicle.app"
    else
        warn "Chronicle GUI application not found"
    fi
    
    log "System GUI application installation completed"
}

# Install system configuration
install_system_config() {
    if [ "$INSTALL_CONFIG" = false ]; then
        return 0
    fi
    
    log "Installing system configuration..."
    
    # Install example configuration
    if [ -f "$SOURCE_DIR/config/chronicle.toml.example" ]; then
        cp "$SOURCE_DIR/config/chronicle.toml.example" "$SYSTEM_CONFIG_DIR/chronicle.toml.example"
        
        if [ ! -f "$SYSTEM_CONFIG_DIR/chronicle.toml" ]; then
            cp "$SOURCE_DIR/config/chronicle.toml.example" "$SYSTEM_CONFIG_DIR/chronicle.toml"
            info "Installed configuration template to $SYSTEM_CONFIG_DIR/chronicle.toml"
        else
            info "Updated configuration example in $SYSTEM_CONFIG_DIR/chronicle.toml.example"
        fi
        
        chown root:wheel "$SYSTEM_CONFIG_DIR/chronicle.toml"*
        chmod 644 "$SYSTEM_CONFIG_DIR/chronicle.toml"*
    fi
    
    # Install documentation
    local docs_dir="$SYSTEM_SHARE_DIR/docs"
    mkdir -p "$docs_dir"
    
    if [ -f "$SOURCE_DIR/README.md" ]; then
        cp "$SOURCE_DIR/README.md" "$docs_dir/"
    fi
    
    if [ -f "$SOURCE_DIR/LICENSE" ]; then
        cp "$SOURCE_DIR/LICENSE" "$docs_dir/"
    fi
    
    if [ -d "$SOURCE_DIR/docs" ]; then
        cp -R "$SOURCE_DIR/docs"/* "$docs_dir/" 2>/dev/null || true
    fi
    
    # Set proper ownership and permissions
    chown -R root:wheel "$SYSTEM_SHARE_DIR"
    find "$SYSTEM_SHARE_DIR" -type d -exec chmod 755 {} \;
    find "$SYSTEM_SHARE_DIR" -type f -exec chmod 644 {} \;
    
    log "System configuration installation completed"
}

# Create launch daemon
create_launch_daemon() {
    if [ "$CREATE_DAEMON" = false ]; then
        return 0
    fi
    
    log "Creating launch daemon..."
    
    cat > "$DAEMON_PLIST" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.chronicle.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>$SYSTEM_BIN_DIR/chronicle</string>
        <string>daemon</string>
        <string>--config</string>
        <string>$SYSTEM_CONFIG_DIR/chronicle.toml</string>
    </array>
    <key>RunAtLoad</key>
    <false/>
    <key>KeepAlive</key>
    <false/>
    <key>UserName</key>
    <string>root</string>
    <key>GroupName</key>
    <string>wheel</string>
    <key>StandardOutPath</key>
    <string>/var/log/chronicle.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/chronicle.error.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin</string>
    </dict>
</dict>
</plist>
EOF
    
    # Set proper ownership and permissions
    chown root:wheel "$DAEMON_PLIST"
    chmod 644 "$DAEMON_PLIST"
    
    # Load daemon (but don't start it automatically)
    launchctl load "$DAEMON_PLIST"
    
    info "Created launch daemon: $DAEMON_PLIST"
    info "To start daemon: sudo launchctl start com.chronicle.daemon"
    info "To enable at boot: sudo launchctl enable system/com.chronicle.daemon"
    
    log "Launch daemon creation completed"
}

# Setup system PATH
setup_system_path() {
    if [ "$INSTALL_CLI" = false ]; then
        return 0
    fi
    
    log "Setting up system PATH..."
    
    # Update /etc/paths.d
    local paths_file="/etc/paths.d/chronicle"
    
    if [ ! -f "$paths_file" ]; then
        echo "$SYSTEM_BIN_DIR" > "$paths_file"
        chmod 644 "$paths_file"
        info "Added $SYSTEM_BIN_DIR to system PATH via $paths_file"
    fi
    
    # Update common shell profiles
    local shell_profiles=("/etc/profile" "/etc/bashrc" "/etc/zshrc")
    local path_line="export PATH=\"$SYSTEM_BIN_DIR:\$PATH\""
    
    for profile in "${shell_profiles[@]}"; do
        if [ -f "$profile" ]; then
            if ! grep -q "$SYSTEM_BIN_DIR" "$profile"; then
                echo "" >> "$profile"
                echo "# Chronicle CLI tools" >> "$profile"
                echo "$path_line" >> "$profile"
                info "Added $SYSTEM_BIN_DIR to PATH in $profile"
            fi
        fi
    done
    
    log "System PATH setup completed"
}

# Create uninstall script
create_uninstall_script() {
    log "Creating uninstall script..."
    
    local uninstall_script="$SYSTEM_BIN_DIR/chronicle-uninstall"
    
    cat > "$uninstall_script" << 'EOF'
#!/bin/bash

# Chronicle System Uninstall Script
# Generated automatically during installation

set -e

echo "Uninstalling Chronicle..."

# Stop and unload daemon
if [ -f "/Library/LaunchDaemons/com.chronicle.daemon.plist" ]; then
    echo "Stopping daemon..."
    launchctl unload "/Library/LaunchDaemons/com.chronicle.daemon.plist" 2>/dev/null || true
    rm -f "/Library/LaunchDaemons/com.chronicle.daemon.plist"
fi

# Remove binaries
echo "Removing binaries..."
rm -f "/usr/local/bin/chronicle"
rm -f "/usr/local/bin/chronicle-packer"
rm -f "/usr/local/bin/chronicle-uninstall"

# Remove libraries
echo "Removing libraries..."
rm -f "/usr/local/lib/libringbuffer.a"
rm -rf "/usr/local/lib/ChronicleCollectors.framework"

# Remove application
echo "Removing application..."
rm -rf "/Applications/Chronicle.app"

# Remove configuration (ask user)
read -p "Remove configuration files? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf "/usr/local/etc/chronicle"
    rm -rf "/usr/local/share/chronicle"
fi

# Remove PATH entry
rm -f "/etc/paths.d/chronicle"

echo "Chronicle uninstallation completed"
EOF
    
    chmod 755 "$uninstall_script"
    chown root:wheel "$uninstall_script"
    
    info "Created uninstall script: $uninstall_script"
    
    log "Uninstall script creation completed"
}

# Verify system installation
verify_system_installation() {
    log "Verifying system installation..."
    
    local verification_failed=false
    
    # Verify CLI tools
    if [ "$INSTALL_CLI" = true ]; then
        if [ -f "$SYSTEM_BIN_DIR/chronicle" ]; then
            if "$SYSTEM_BIN_DIR/chronicle" --version &> /dev/null; then
                info "Chronicle CLI: OK"
            else
                warn "Chronicle CLI: INSTALLED but not working"
                verification_failed=true
            fi
        else
            warn "Chronicle CLI: NOT INSTALLED"
            verification_failed=true
        fi
        
        if [ -f "$SYSTEM_BIN_DIR/chronicle-packer" ]; then
            if "$SYSTEM_BIN_DIR/chronicle-packer" --help &> /dev/null; then
                info "Chronicle Packer: OK"
            else
                warn "Chronicle Packer: INSTALLED but not working"
                verification_failed=true
            fi
        else
            warn "Chronicle Packer: NOT INSTALLED"
            verification_failed=true
        fi
    fi
    
    # Verify GUI app
    if [ "$INSTALL_APP" = true ]; then
        if [ -d "$APPLICATIONS_DIR/Chronicle.app" ]; then
            info "Chronicle GUI: OK"
        else
            warn "Chronicle GUI: NOT INSTALLED"
            verification_failed=true
        fi
    fi
    
    # Verify configuration
    if [ "$INSTALL_CONFIG" = true ]; then
        if [ -f "$SYSTEM_CONFIG_DIR/chronicle.toml" ]; then
            info "Configuration: OK"
        else
            warn "Configuration: NOT INSTALLED"
            verification_failed=true
        fi
    fi
    
    # Verify daemon
    if [ "$CREATE_DAEMON" = true ]; then
        if [ -f "$DAEMON_PLIST" ]; then
            info "Launch Daemon: OK"
        else
            warn "Launch Daemon: NOT INSTALLED"
            verification_failed=true
        fi
    fi
    
    if [ "$verification_failed" = true ]; then
        warn "Installation verification found issues"
        return 1
    else
        log "Installation verification passed"
        return 0
    fi
}

# Generate system installation report
generate_system_install_report() {
    log "Generating system installation report..."
    
    local report_file="$SYSTEM_SHARE_DIR/installation_report.txt"
    
    cat > "$report_file" << EOF
Chronicle System Installation Report
Generated: $(date)

Installation Configuration:
  Source Directory: $SOURCE_DIR
  Install GUI App: $INSTALL_APP
  Install CLI Tools: $INSTALL_CLI
  Install Configuration: $INSTALL_CONFIG
  Create Launch Daemon: $CREATE_DAEMON
  Force Install: $FORCE_INSTALL

System Paths:
  Binaries: $SYSTEM_BIN_DIR
  Libraries: $SYSTEM_LIB_DIR
  Configuration: $SYSTEM_CONFIG_DIR
  Documentation: $SYSTEM_SHARE_DIR
  Application: $APPLICATIONS_DIR
  Launch Daemon: $DAEMON_PLIST

Installed Components:
EOF
    
    if [ "$INSTALL_CLI" = true ]; then
        echo "  CLI Tools:" >> "$report_file"
        if [ -f "$SYSTEM_BIN_DIR/chronicle" ]; then
            echo "    chronicle: $SYSTEM_BIN_DIR/chronicle" >> "$report_file"
        fi
        if [ -f "$SYSTEM_BIN_DIR/chronicle-packer" ]; then
            echo "    chronicle-packer: $SYSTEM_BIN_DIR/chronicle-packer" >> "$report_file"
        fi
    fi
    
    if [ "$INSTALL_APP" = true ]; then
        echo "  GUI Application:" >> "$report_file"
        if [ -d "$APPLICATIONS_DIR/Chronicle.app" ]; then
            echo "    Chronicle.app: $APPLICATIONS_DIR/Chronicle.app" >> "$report_file"
        fi
    fi
    
    if [ "$INSTALL_CONFIG" = true ]; then
        echo "  Configuration:" >> "$report_file"
        if [ -f "$SYSTEM_CONFIG_DIR/chronicle.toml" ]; then
            echo "    Configuration: $SYSTEM_CONFIG_DIR/chronicle.toml" >> "$report_file"
        fi
        if [ -d "$SYSTEM_SHARE_DIR/docs" ]; then
            echo "    Documentation: $SYSTEM_SHARE_DIR/docs" >> "$report_file"
        fi
    fi
    
    if [ "$CREATE_DAEMON" = true ]; then
        echo "  Launch Daemon:" >> "$report_file"
        if [ -f "$DAEMON_PLIST" ]; then
            echo "    Daemon: $DAEMON_PLIST" >> "$report_file"
        fi
    fi
    
    echo "" >> "$report_file"
    echo "Management Commands:" >> "$report_file"
    echo "  Start daemon: sudo launchctl start com.chronicle.daemon" >> "$report_file"
    echo "  Stop daemon: sudo launchctl stop com.chronicle.daemon" >> "$report_file"
    echo "  Enable at boot: sudo launchctl enable system/com.chronicle.daemon" >> "$report_file"
    echo "  Uninstall: sudo chronicle-uninstall" >> "$report_file"
    
    chown root:wheel "$report_file"
    chmod 644 "$report_file"
    
    log "System installation report saved to: $report_file"
}

# Main system installation function
main() {
    log "Starting Chronicle system installation..."
    
    check_root
    parse_args "$@"
    
    info "System installation configuration:"
    info "  Source Directory: $SOURCE_DIR"
    info "  Install GUI App: $INSTALL_APP"
    info "  Install CLI Tools: $INSTALL_CLI"
    info "  Install Configuration: $INSTALL_CONFIG"
    info "  Create Launch Daemon: $CREATE_DAEMON"
    info "  Force Install: $FORCE_INSTALL"
    
    check_install_prerequisites
    check_existing_installation
    create_system_directories
    install_system_cli_tools
    install_system_gui_app
    install_system_config
    create_launch_daemon
    setup_system_path
    create_uninstall_script
    
    if verify_system_installation; then
        generate_system_install_report
        log "Chronicle system installation completed successfully!"
        
        echo
        info "Installation Summary:"
        if [ "$INSTALL_CLI" = true ]; then
            info "  CLI tools installed to: $SYSTEM_BIN_DIR"
        fi
        if [ "$INSTALL_APP" = true ]; then
            info "  GUI app installed to: $APPLICATIONS_DIR/Chronicle.app"
        fi
        if [ "$INSTALL_CONFIG" = true ]; then
            info "  Configuration in: $SYSTEM_CONFIG_DIR"
        fi
        if [ "$CREATE_DAEMON" = true ]; then
            info "  Launch daemon created: $DAEMON_PLIST"
        fi
        
        echo
        info "Management Commands:"
        if [ "$CREATE_DAEMON" = true ]; then
            info "  Start daemon: sudo launchctl start com.chronicle.daemon"
            info "  Stop daemon: sudo launchctl stop com.chronicle.daemon"
        fi
        info "  Uninstall: sudo chronicle-uninstall"
        
        echo
        info "Next steps:"
        info "  1. All users can now use 'chronicle' command"
        info "  2. Launch Chronicle from Applications folder"
        if [ "$CREATE_DAEMON" = true ]; then
            info "  3. Configure and start daemon if needed"
        fi
    else
        error "Installation verification failed"
    fi
}

# Run main function
main "$@"