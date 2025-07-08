#!/bin/bash

# Chronicle Local Installation Script
# Installs Chronicle for the current user

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
INSTALL_DIR="$HOME/.local"
CONFIG_DIR="$HOME/.config/chronicle"
DATA_DIR="$HOME/.local/share/chronicle"
BIN_DIR="$HOME/.local/bin"
SOURCE_DIR=""
INSTALL_APP=true
INSTALL_CLI=true
INSTALL_CONFIG=true

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

Install Chronicle locally for the current user.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -f, --force             Force installation (overwrite existing)
    --source-dir DIR        Source directory (auto-detected if not specified)
    --install-dir DIR       Installation directory (default: ~/.local)
    --config-dir DIR        Configuration directory (default: ~/.config/chronicle)
    --data-dir DIR          Data directory (default: ~/.local/share/chronicle)
    --bin-dir DIR           Binary directory (default: ~/.local/bin)
    --no-app                Don't install GUI application
    --no-cli                Don't install CLI tools
    --no-config             Don't install configuration files
    --app-only              Install only GUI application
    --cli-only              Install only CLI tools

EXAMPLES:
    $0                      # Full installation with auto-detection
    $0 --source-dir /path/to/chronicle
    $0 --cli-only           # Install CLI tools only
    $0 --force              # Force reinstallation

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
                FORCE_INSTALL=true
                shift
                ;;
            --source-dir)
                SOURCE_DIR="$2"
                shift 2
                ;;
            --install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --config-dir)
                CONFIG_DIR="$2"
                shift 2
                ;;
            --data-dir)
                DATA_DIR="$2"
                shift 2
                ;;
            --bin-dir)
                BIN_DIR="$2"
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
            --app-only)
                INSTALL_APP=true
                INSTALL_CLI=false
                INSTALL_CONFIG=false
                shift
                ;;
            --cli-only)
                INSTALL_APP=false
                INSTALL_CLI=true
                INSTALL_CONFIG=true
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
        # Try to find source directory
        local script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
        local potential_source="$(dirname "$(dirname "$script_dir")")"
        
        if [ -f "$potential_source/Cargo.toml" ] || [ -f "$potential_source/Chronicle.xcworkspace" ]; then
            SOURCE_DIR="$potential_source"
            info "Auto-detected source directory: $SOURCE_DIR"
        else
            error "Cannot auto-detect source directory. Use --source-dir to specify."
        fi
    fi
    
    # Make paths absolute
    SOURCE_DIR="$(cd "$SOURCE_DIR" && pwd)"
    INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"
    CONFIG_DIR="${CONFIG_DIR/#\~/$HOME}"
    DATA_DIR="${DATA_DIR/#\~/$HOME}"
    BIN_DIR="${BIN_DIR/#\~/$HOME}"
}

# Check installation prerequisites
check_install_prerequisites() {
    log "Checking installation prerequisites..."
    
    # Check if source directory exists
    if [ ! -d "$SOURCE_DIR" ]; then
        error "Source directory not found: $SOURCE_DIR"
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
        warn "No built binaries found. Building Chronicle..."
        if ! build_chronicle; then
            error "Failed to build Chronicle"
        fi
    fi
    
    log "Prerequisites check completed"
}

# Build Chronicle if needed
build_chronicle() {
    log "Building Chronicle..."
    
    cd "$SOURCE_DIR"
    
    # Check for build script
    if [ -f "scripts/dev/build.sh" ]; then
        if ! ./scripts/dev/build.sh; then
            return 1
        fi
    else
        # Manual build
        if [ "$INSTALL_CLI" = true ]; then
            if [ -d "cli" ]; then
                cd cli
                cargo build --release
                cd ..
            fi
            
            if [ -d "packer" ]; then
                cd packer
                cargo build --release
                cd ..
            fi
        fi
        
        if [ "$INSTALL_APP" = true ]; then
            if [ -f "Chronicle.xcworkspace" ]; then
                xcodebuild -workspace Chronicle.xcworkspace -scheme ChronicleUI -configuration Release build
            fi
        fi
    fi
    
    log "Build completed"
    return 0
}

# Check for existing installation
check_existing_installation() {
    log "Checking for existing installation..."
    
    local existing_files=()
    
    # Check for existing binaries
    if [ -f "$BIN_DIR/chronicle" ]; then
        existing_files+=("$BIN_DIR/chronicle")
    fi
    
    if [ -f "$BIN_DIR/chronicle-packer" ]; then
        existing_files+=("$BIN_DIR/chronicle-packer")
    fi
    
    # Check for existing app
    if [ -d "$HOME/Applications/Chronicle.app" ]; then
        existing_files+=("$HOME/Applications/Chronicle.app")
    fi
    
    if [ -d "/Applications/Chronicle.app" ]; then
        existing_files+=("/Applications/Chronicle.app")
    fi
    
    # Check for existing config
    if [ -d "$CONFIG_DIR" ]; then
        existing_files+=("$CONFIG_DIR")
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
    fi
    
    log "Existing installation check completed"
}

# Create installation directories
create_install_directories() {
    log "Creating installation directories..."
    
    local directories=("$INSTALL_DIR" "$CONFIG_DIR" "$DATA_DIR" "$BIN_DIR")
    
    if [ "$INSTALL_APP" = true ]; then
        directories+=("$HOME/Applications")
    fi
    
    for dir in "${directories[@]}"; do
        if [ ! -d "$dir" ]; then
            mkdir -p "$dir"
            if [ "$VERBOSE" = true ]; then
                info "Created directory: $dir"
            fi
        fi
    done
    
    log "Installation directories created"
}

# Install CLI tools
install_cli_tools() {
    if [ "$INSTALL_CLI" = false ]; then
        return 0
    fi
    
    log "Installing CLI tools..."
    
    # Find and install chronicle binary
    local chronicle_bin=""
    if [ -f "$SOURCE_DIR/cli/target/release/chronicle" ]; then
        chronicle_bin="$SOURCE_DIR/cli/target/release/chronicle"
    elif [ -f "$SOURCE_DIR/build/release/cli/chronicle" ]; then
        chronicle_bin="$SOURCE_DIR/build/release/cli/chronicle"
    fi
    
    if [ -n "$chronicle_bin" ]; then
        cp "$chronicle_bin" "$BIN_DIR/chronicle"
        chmod +x "$BIN_DIR/chronicle"
        info "Installed chronicle CLI to $BIN_DIR/chronicle"
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
        cp "$packer_bin" "$BIN_DIR/chronicle-packer"
        chmod +x "$BIN_DIR/chronicle-packer"
        info "Installed chronicle-packer to $BIN_DIR/chronicle-packer"
    else
        warn "Chronicle packer binary not found"
    fi
    
    log "CLI tools installation completed"
}

# Install GUI application
install_gui_app() {
    if [ "$INSTALL_APP" = false ]; then
        return 0
    fi
    
    log "Installing GUI application..."
    
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
        if [ -d "$HOME/Applications/Chronicle.app" ]; then
            rm -rf "$HOME/Applications/Chronicle.app"
        fi
        
        # Copy app bundle
        cp -R "$app_bundle" "$HOME/Applications/Chronicle.app"
        info "Installed Chronicle.app to $HOME/Applications/Chronicle.app"
    else
        warn "Chronicle GUI application not found"
    fi
    
    log "GUI application installation completed"
}

# Install configuration files
install_config_files() {
    if [ "$INSTALL_CONFIG" = false ]; then
        return 0
    fi
    
    log "Installing configuration files..."
    
    # Install example configuration
    if [ -f "$SOURCE_DIR/config/chronicle.toml.example" ]; then
        if [ ! -f "$CONFIG_DIR/chronicle.toml" ]; then
            cp "$SOURCE_DIR/config/chronicle.toml.example" "$CONFIG_DIR/chronicle.toml"
            info "Installed configuration template to $CONFIG_DIR/chronicle.toml"
        else
            cp "$SOURCE_DIR/config/chronicle.toml.example" "$CONFIG_DIR/chronicle.toml.example"
            info "Updated configuration example in $CONFIG_DIR/chronicle.toml.example"
        fi
    fi
    
    # Install documentation
    local docs_dir="$DATA_DIR/docs"
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
    
    log "Configuration files installation completed"
}

# Setup PATH
setup_path() {
    if [ "$INSTALL_CLI" = false ]; then
        return 0
    fi
    
    log "Setting up PATH..."
    
    # Check if BIN_DIR is already in PATH
    if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
        # Add to shell profiles
        local shell_profiles=("$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile")
        local path_line="export PATH=\"$BIN_DIR:\$PATH\""
        
        for profile in "${shell_profiles[@]}"; do
            if [ -f "$profile" ]; then
                if ! grep -q "$BIN_DIR" "$profile"; then
                    echo "" >> "$profile"
                    echo "# Chronicle CLI tools" >> "$profile"
                    echo "$path_line" >> "$profile"
                    info "Added $BIN_DIR to PATH in $profile"
                fi
            fi
        done
        
        # For the current session
        export PATH="$BIN_DIR:$PATH"
        
        warn "Please restart your terminal or run 'source ~/.bashrc' (or ~/.zshrc) to update PATH"
    else
        info "PATH already contains $BIN_DIR"
    fi
    
    log "PATH setup completed"
}

# Create desktop entry (Linux-style for compatibility)
create_desktop_entry() {
    if [ "$INSTALL_APP" = false ]; then
        return 0
    fi
    
    # This is primarily for Linux, but we'll create it for consistency
    local desktop_dir="$HOME/.local/share/applications"
    mkdir -p "$desktop_dir"
    
    local desktop_file="$desktop_dir/chronicle.desktop"
    
    cat > "$desktop_file" << EOF
[Desktop Entry]
Name=Chronicle
Comment=Activity monitoring and data management
Exec=$HOME/Applications/Chronicle.app/Contents/MacOS/ChronicleUI
Icon=chronicle
Type=Application
Categories=Utility;System;
StartupNotify=true
EOF
    
    if [ "$VERBOSE" = true ]; then
        info "Created desktop entry: $desktop_file"
    fi
}

# Verify installation
verify_installation() {
    log "Verifying installation..."
    
    local verification_failed=false
    
    # Verify CLI tools
    if [ "$INSTALL_CLI" = true ]; then
        if [ -f "$BIN_DIR/chronicle" ]; then
            if "$BIN_DIR/chronicle" --version &> /dev/null; then
                info "Chronicle CLI: OK"
            else
                warn "Chronicle CLI: INSTALLED but not working"
                verification_failed=true
            fi
        else
            warn "Chronicle CLI: NOT INSTALLED"
            verification_failed=true
        fi
        
        if [ -f "$BIN_DIR/chronicle-packer" ]; then
            if "$BIN_DIR/chronicle-packer" --help &> /dev/null; then
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
        if [ -d "$HOME/Applications/Chronicle.app" ]; then
            info "Chronicle GUI: OK"
        else
            warn "Chronicle GUI: NOT INSTALLED"
            verification_failed=true
        fi
    fi
    
    # Verify configuration
    if [ "$INSTALL_CONFIG" = true ]; then
        if [ -f "$CONFIG_DIR/chronicle.toml" ]; then
            info "Configuration: OK"
        else
            warn "Configuration: NOT INSTALLED"
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

# Generate installation report
generate_install_report() {
    log "Generating installation report..."
    
    local report_file="$DATA_DIR/installation_report.txt"
    mkdir -p "$(dirname "$report_file")"
    
    cat > "$report_file" << EOF
Chronicle Local Installation Report
Generated: $(date)

Installation Configuration:
  Source Directory: $SOURCE_DIR
  Install Directory: $INSTALL_DIR
  Configuration Directory: $CONFIG_DIR
  Data Directory: $DATA_DIR
  Binary Directory: $BIN_DIR
  
  Install GUI App: $INSTALL_APP
  Install CLI Tools: $INSTALL_CLI
  Install Configuration: $INSTALL_CONFIG
  Force Install: $FORCE_INSTALL

Installed Components:
EOF
    
    if [ "$INSTALL_CLI" = true ]; then
        echo "  CLI Tools:" >> "$report_file"
        if [ -f "$BIN_DIR/chronicle" ]; then
            echo "    chronicle: $BIN_DIR/chronicle" >> "$report_file"
        fi
        if [ -f "$BIN_DIR/chronicle-packer" ]; then
            echo "    chronicle-packer: $BIN_DIR/chronicle-packer" >> "$report_file"
        fi
    fi
    
    if [ "$INSTALL_APP" = true ]; then
        echo "  GUI Application:" >> "$report_file"
        if [ -d "$HOME/Applications/Chronicle.app" ]; then
            echo "    Chronicle.app: $HOME/Applications/Chronicle.app" >> "$report_file"
        fi
    fi
    
    if [ "$INSTALL_CONFIG" = true ]; then
        echo "  Configuration:" >> "$report_file"
        if [ -f "$CONFIG_DIR/chronicle.toml" ]; then
            echo "    Configuration: $CONFIG_DIR/chronicle.toml" >> "$report_file"
        fi
        if [ -d "$DATA_DIR/docs" ]; then
            echo "    Documentation: $DATA_DIR/docs" >> "$report_file"
        fi
    fi
    
    echo "" >> "$report_file"
    echo "Usage:" >> "$report_file"
    
    if [ "$INSTALL_CLI" = true ]; then
        echo "  CLI: chronicle --help" >> "$report_file"
        echo "       chronicle-packer --help" >> "$report_file"
    fi
    
    if [ "$INSTALL_APP" = true ]; then
        echo "  GUI: Open Chronicle from Applications folder" >> "$report_file"
    fi
    
    log "Installation report saved to: $report_file"
}

# Main installation function
main() {
    log "Starting Chronicle local installation..."
    
    parse_args "$@"
    
    info "Installation configuration:"
    info "  Source Directory: $SOURCE_DIR"
    info "  Install Directory: $INSTALL_DIR"
    info "  Configuration Directory: $CONFIG_DIR"
    info "  Binary Directory: $BIN_DIR"
    info "  Install GUI App: $INSTALL_APP"
    info "  Install CLI Tools: $INSTALL_CLI"
    info "  Install Configuration: $INSTALL_CONFIG"
    info "  Force Install: $FORCE_INSTALL"
    
    check_install_prerequisites
    check_existing_installation
    create_install_directories
    install_cli_tools
    install_gui_app
    install_config_files
    setup_path
    create_desktop_entry
    
    if verify_installation; then
        generate_install_report
        log "Chronicle local installation completed successfully!"
        
        echo
        info "Installation Summary:"
        if [ "$INSTALL_CLI" = true ]; then
            info "  CLI tools installed to: $BIN_DIR"
        fi
        if [ "$INSTALL_APP" = true ]; then
            info "  GUI app installed to: $HOME/Applications/Chronicle.app"
        fi
        if [ "$INSTALL_CONFIG" = true ]; then
            info "  Configuration in: $CONFIG_DIR"
        fi
        
        echo
        info "Next steps:"
        if [ "$INSTALL_CLI" = true ]; then
            info "  1. Restart your terminal or run 'source ~/.bashrc'"
            info "  2. Try: chronicle --help"
        fi
        if [ "$INSTALL_APP" = true ]; then
            info "  3. Launch Chronicle from Applications folder"
        fi
    else
        error "Installation verification failed"
    fi
}

# Run main function
main "$@"