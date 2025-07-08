#!/bin/bash

# Chronicle ZIP Distribution Creation Script
# Creates ZIP archives for different distribution channels

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
OUTPUT_DIR="$ROOT_DIR/dist/zip"
DISTRIBUTION_TYPE="full"
INCLUDE_SOURCES=false
COMPRESSION_LEVEL=6

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

Create ZIP distribution packages for Chronicle.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    --version VERSION       Application version (required)
    --app-name NAME         Application name (default: Chronicle)
    --build-dir DIR         Build directory (default: build/release)
    --output-dir DIR        Output directory (default: dist/zip)
    --type TYPE             Distribution type (full, app-only, cli-only, dev)
    --include-sources       Include source code (for dev distribution)
    --compression LEVEL     Compression level 1-9 (default: 6)

DISTRIBUTION TYPES:
    full                    Complete distribution with app and CLI tools
    app-only                GUI application only
    cli-only                Command-line tools only
    dev                     Development distribution with sources

EXAMPLES:
    $0 --version 1.0.0
    $0 --version 1.0.0 --type app-only
    $0 --version 1.0.0 --type dev --include-sources

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
            --type)
                DISTRIBUTION_TYPE="$2"
                shift 2
                ;;
            --include-sources)
                INCLUDE_SOURCES=true
                shift
                ;;
            --compression)
                COMPRESSION_LEVEL="$2"
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
    
    # Validate required parameters
    if [ -z "$VERSION" ]; then
        error "Version is required. Use --version to specify."
    fi
    
    # Validate distribution type
    case $DISTRIBUTION_TYPE in
        "full"|"app-only"|"cli-only"|"dev")
            ;;
        *)
            error "Invalid distribution type: $DISTRIBUTION_TYPE. Use: full, app-only, cli-only, dev"
            ;;
    esac
    
    # Validate compression level
    if [[ ! "$COMPRESSION_LEVEL" =~ ^[1-9]$ ]]; then
        error "Invalid compression level: $COMPRESSION_LEVEL. Use 1-9."
    fi
    
    # Make paths absolute
    if [[ ! "$BUILD_DIR" = /* ]]; then
        BUILD_DIR="$ROOT_DIR/$BUILD_DIR"
    fi
    
    if [[ ! "$OUTPUT_DIR" = /* ]]; then
        OUTPUT_DIR="$ROOT_DIR/$OUTPUT_DIR"
    fi
}

# Check ZIP creation prerequisites
check_zip_prerequisites() {
    log "Checking ZIP creation prerequisites..."
    
    # Check for required tools
    if ! command -v zip &> /dev/null; then
        error "zip command not found"
    fi
    
    # Check for build artifacts based on distribution type
    case $DISTRIBUTION_TYPE in
        "full"|"app-only")
            # Check for app bundle
            local app_found=false
            if [ -d "$BUILD_DIR/ChronicleUI.app" ]; then
                app_found=true
            elif [ -d "$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app" ]; then
                app_found=true
            fi
            
            if [ "$app_found" = false ]; then
                error "App bundle not found in build directory"
            fi
            ;;
    esac
    
    case $DISTRIBUTION_TYPE in
        "full"|"cli-only")
            # Check for CLI tools
            if [ ! -f "$BUILD_DIR/cli/chronicle" ] && [ ! -f "$BUILD_DIR/packer/chronicle-packer" ]; then
                error "CLI tools not found in build directory"
            fi
            ;;
    esac
    
    log "Prerequisites check completed"
}

# Prepare full distribution
prepare_full_distribution() {
    local dist_dir="$1"
    
    log "Preparing full distribution..."
    
    # Copy app bundle
    local app_bundle=""
    if [ -d "$BUILD_DIR/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/ChronicleUI.app"
    elif [ -d "$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app"
    fi
    
    if [ -n "$app_bundle" ]; then
        cp -R "$app_bundle" "$dist_dir/$APP_NAME.app"
    fi
    
    # Copy CLI tools
    local cli_dir="$dist_dir/bin"
    mkdir -p "$cli_dir"
    
    if [ -f "$BUILD_DIR/cli/chronicle" ]; then
        cp "$BUILD_DIR/cli/chronicle" "$cli_dir/"
    fi
    
    if [ -f "$BUILD_DIR/packer/chronicle-packer" ]; then
        cp "$BUILD_DIR/packer/chronicle-packer" "$cli_dir/"
    fi
    
    # Copy frameworks
    local frameworks_dir="$dist_dir/Frameworks"
    if [ -d "$BUILD_DIR/xcode/Build/Products/Release/ChronicleCollectors.framework" ]; then
        mkdir -p "$frameworks_dir"
        cp -R "$BUILD_DIR/xcode/Build/Products/Release/ChronicleCollectors.framework" "$frameworks_dir/"
    fi
    
    # Copy configuration
    local config_dir="$dist_dir/config"
    mkdir -p "$config_dir"
    
    if [ -f "$ROOT_DIR/config/chronicle.toml.example" ]; then
        cp "$ROOT_DIR/config/chronicle.toml.example" "$config_dir/"
    fi
    
    # Copy documentation
    local docs_dir="$dist_dir/docs"
    mkdir -p "$docs_dir"
    
    if [ -f "$ROOT_DIR/README.md" ]; then
        cp "$ROOT_DIR/README.md" "$docs_dir/"
    fi
    
    if [ -f "$ROOT_DIR/LICENSE" ]; then
        cp "$ROOT_DIR/LICENSE" "$docs_dir/"
    fi
    
    # Create installation script
    create_installation_script "$dist_dir"
    
    log "Full distribution prepared"
}

# Prepare app-only distribution
prepare_app_only_distribution() {
    local dist_dir="$1"
    
    log "Preparing app-only distribution..."
    
    # Copy app bundle
    local app_bundle=""
    if [ -d "$BUILD_DIR/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/ChronicleUI.app"
    elif [ -d "$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app" ]; then
        app_bundle="$BUILD_DIR/xcode/Build/Products/Release/ChronicleUI.app"
    fi
    
    if [ -n "$app_bundle" ]; then
        cp -R "$app_bundle" "$dist_dir/$APP_NAME.app"
    fi
    
    # Copy minimal documentation
    if [ -f "$ROOT_DIR/README.md" ]; then
        cp "$ROOT_DIR/README.md" "$dist_dir/"
    fi
    
    if [ -f "$ROOT_DIR/LICENSE" ]; then
        cp "$ROOT_DIR/LICENSE" "$dist_dir/"
    fi
    
    # Create simple installation instructions
    cat > "$dist_dir/INSTALL.txt" << EOF
Chronicle Installation Instructions

To install Chronicle:
1. Copy Chronicle.app to your Applications folder
2. Launch Chronicle from Applications folder

For more information, see README.md
EOF
    
    log "App-only distribution prepared"
}

# Prepare CLI-only distribution
prepare_cli_only_distribution() {
    local dist_dir="$1"
    
    log "Preparing CLI-only distribution..."
    
    # Copy CLI tools
    local bin_dir="$dist_dir/bin"
    mkdir -p "$bin_dir"
    
    if [ -f "$BUILD_DIR/cli/chronicle" ]; then
        cp "$BUILD_DIR/cli/chronicle" "$bin_dir/"
    fi
    
    if [ -f "$BUILD_DIR/packer/chronicle-packer" ]; then
        cp "$BUILD_DIR/packer/chronicle-packer" "$bin_dir/"
    fi
    
    # Copy ring buffer library
    if [ -f "$BUILD_DIR/libringbuffer.a" ]; then
        mkdir -p "$dist_dir/lib"
        cp "$BUILD_DIR/libringbuffer.a" "$dist_dir/lib/"
    fi
    
    # Copy configuration
    local config_dir="$dist_dir/config"
    mkdir -p "$config_dir"
    
    if [ -f "$ROOT_DIR/config/chronicle.toml.example" ]; then
        cp "$ROOT_DIR/config/chronicle.toml.example" "$config_dir/"
    fi
    
    # Copy documentation
    if [ -f "$ROOT_DIR/README.md" ]; then
        cp "$ROOT_DIR/README.md" "$dist_dir/"
    fi
    
    if [ -f "$ROOT_DIR/LICENSE" ]; then
        cp "$ROOT_DIR/LICENSE" "$dist_dir/"
    fi
    
    # Create CLI installation script
    create_cli_installation_script "$dist_dir"
    
    log "CLI-only distribution prepared"
}

# Prepare development distribution
prepare_dev_distribution() {
    local dist_dir="$1"
    
    log "Preparing development distribution..."
    
    # Copy build artifacts
    if [ -d "$BUILD_DIR" ]; then
        mkdir -p "$dist_dir/build"
        # Copy only essential build artifacts, not everything
        for item in "cli" "packer" "*.a" "xcode/Build/Products"; do
            if [ -e "$BUILD_DIR/$item" ]; then
                cp -R "$BUILD_DIR/$item" "$dist_dir/build/" 2>/dev/null || true
            fi
        done
    fi
    
    # Copy sources if requested
    if [ "$INCLUDE_SOURCES" = true ]; then
        local src_dir="$dist_dir/src"
        mkdir -p "$src_dir"
        
        # Copy source directories
        for src in "cli" "packer" "collectors" "ui" "ring-buffer" "benchmarks" "tests"; do
            if [ -d "$ROOT_DIR/$src" ]; then
                cp -R "$ROOT_DIR/$src" "$src_dir/"
            fi
        done
        
        # Copy configuration and build files
        for file in "Cargo.toml" "Cargo.lock" "Makefile" "Chronicle.xcworkspace"; do
            if [ -e "$ROOT_DIR/$file" ]; then
                cp -R "$ROOT_DIR/$file" "$src_dir/"
            fi
        done
        
        # Copy scripts
        if [ -d "$ROOT_DIR/scripts" ]; then
            cp -R "$ROOT_DIR/scripts" "$src_dir/"
        fi
    fi
    
    # Copy configuration
    if [ -d "$ROOT_DIR/config" ]; then
        cp -R "$ROOT_DIR/config" "$dist_dir/"
    fi
    
    # Copy documentation
    for doc in "README.md" "LICENSE" "docs" "technical_spec.md"; do
        if [ -e "$ROOT_DIR/$doc" ]; then
            cp -R "$ROOT_DIR/$doc" "$dist_dir/"
        fi
    done
    
    # Create development setup script
    create_dev_setup_script "$dist_dir"
    
    log "Development distribution prepared"
}

# Create installation script
create_installation_script() {
    local dist_dir="$1"
    local install_script="$dist_dir/install.sh"
    
    cat > "$install_script" << 'EOF'
#!/bin/bash
# Chronicle Installation Script

set -e

echo "Installing Chronicle..."

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Install app
if [ -d "$SCRIPT_DIR/Chronicle.app" ]; then
    echo "Installing Chronicle.app..."
    cp -R "$SCRIPT_DIR/Chronicle.app" /Applications/
    echo "Chronicle.app installed to /Applications"
fi

# Install CLI tools
if [ -d "$SCRIPT_DIR/bin" ]; then
    echo "Installing CLI tools..."
    sudo mkdir -p /usr/local/bin
    sudo cp "$SCRIPT_DIR/bin/"* /usr/local/bin/
    sudo chmod +x /usr/local/bin/chronicle*
    echo "CLI tools installed to /usr/local/bin"
fi

# Install frameworks
if [ -d "$SCRIPT_DIR/Frameworks" ]; then
    echo "Installing frameworks..."
    sudo mkdir -p /usr/local/lib
    sudo cp -R "$SCRIPT_DIR/Frameworks/"* /usr/local/lib/
    echo "Frameworks installed to /usr/local/lib"
fi

# Install configuration
if [ -d "$SCRIPT_DIR/config" ]; then
    echo "Installing configuration..."
    mkdir -p "$HOME/.config/chronicle"
    cp "$SCRIPT_DIR/config/"* "$HOME/.config/chronicle/"
    echo "Configuration installed to $HOME/.config/chronicle"
fi

echo "Chronicle installation completed!"
echo "You can now launch Chronicle from Applications or use 'chronicle' command in Terminal"
EOF
    
    chmod +x "$install_script"
}

# Create CLI installation script
create_cli_installation_script() {
    local dist_dir="$1"
    local install_script="$dist_dir/install.sh"
    
    cat > "$install_script" << 'EOF'
#!/bin/bash
# Chronicle CLI Installation Script

set -e

echo "Installing Chronicle CLI tools..."

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Install CLI tools
if [ -d "$SCRIPT_DIR/bin" ]; then
    echo "Installing CLI tools..."
    sudo mkdir -p /usr/local/bin
    sudo cp "$SCRIPT_DIR/bin/"* /usr/local/bin/
    sudo chmod +x /usr/local/bin/chronicle*
    echo "CLI tools installed to /usr/local/bin"
fi

# Install libraries
if [ -d "$SCRIPT_DIR/lib" ]; then
    echo "Installing libraries..."
    sudo mkdir -p /usr/local/lib
    sudo cp "$SCRIPT_DIR/lib/"* /usr/local/lib/
    echo "Libraries installed to /usr/local/lib"
fi

# Install configuration
if [ -d "$SCRIPT_DIR/config" ]; then
    echo "Installing configuration..."
    mkdir -p "$HOME/.config/chronicle"
    cp "$SCRIPT_DIR/config/"* "$HOME/.config/chronicle/"
    echo "Configuration installed to $HOME/.config/chronicle"
fi

echo "Chronicle CLI installation completed!"
echo "You can now use 'chronicle' command in Terminal"
EOF
    
    chmod +x "$install_script"
}

# Create development setup script
create_dev_setup_script() {
    local dist_dir="$1"
    local setup_script="$dist_dir/dev_setup.sh"
    
    cat > "$setup_script" << 'EOF'
#!/bin/bash
# Chronicle Development Setup Script

set -e

echo "Setting up Chronicle development environment..."

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Check for sources
if [ -d "$SCRIPT_DIR/src" ]; then
    echo "Sources found. Setting up development environment..."
    cd "$SCRIPT_DIR/src"
    
    # Run development setup if available
    if [ -f "scripts/dev/setup.sh" ]; then
        echo "Running development setup..."
        ./scripts/dev/setup.sh
    else
        echo "Manual setup required. See README.md for instructions."
    fi
else
    echo "Sources not included in this distribution."
    echo "Clone the repository for development setup."
fi

echo "Development setup completed!"
EOF
    
    chmod +x "$setup_script"
}

# Create ZIP archive
create_zip_archive() {
    local dist_dir="$1"
    local zip_name="$2"
    local zip_path="$OUTPUT_DIR/$zip_name"
    
    log "Creating ZIP archive: $zip_name"
    
    # Remove existing ZIP
    rm -f "$zip_path"
    
    # Create ZIP with specified compression level
    cd "$(dirname "$dist_dir")"
    local dist_name="$(basename "$dist_dir")"
    
    local zip_flags="-r -$COMPRESSION_LEVEL"
    if [ "$VERBOSE" = true ]; then
        zip_flags="$zip_flags -v"
    else
        zip_flags="$zip_flags -q"
    fi
    
    # Exclude unwanted files
    zip $zip_flags "$zip_path" "$dist_name" \
        -x "*.DS_Store" \
        -x "*/__pycache__/*" \
        -x "*/target/debug/*" \
        -x "*/.git/*" \
        -x "*/node_modules/*"
    
    if [ ! -f "$zip_path" ]; then
        error "ZIP creation failed"
    fi
    
    log "ZIP archive created: $zip_path"
}

# Generate distribution info
generate_distribution_info() {
    local zip_name="$1"
    local zip_path="$OUTPUT_DIR/$zip_name"
    local info_file="$OUTPUT_DIR/${zip_name%.zip}-info.txt"
    
    log "Generating distribution information..."
    
    cat > "$info_file" << EOF
Chronicle Distribution Information
Generated: $(date)

Distribution Details:
  Name: $zip_name
  Type: $DISTRIBUTION_TYPE
  Version: $VERSION
  Compression Level: $COMPRESSION_LEVEL
  
File Information:
  Path: $zip_path
  Size: $(ls -lh "$zip_path" | awk '{print $5}')
  SHA256: $(shasum -a 256 "$zip_path" | awk '{print $1}')
  MD5: $(md5 -q "$zip_path")

Contents:
EOF
    
    # List ZIP contents
    unzip -l "$zip_path" | tail -n +4 | head -n -2 >> "$info_file"
    
    echo "" >> "$info_file"
    echo "Installation:" >> "$info_file"
    
    case $DISTRIBUTION_TYPE in
        "full")
            echo "  1. Extract the ZIP archive" >> "$info_file"
            echo "  2. Run ./install.sh for automated installation" >> "$info_file"
            echo "  3. Or manually copy components to desired locations" >> "$info_file"
            ;;
        "app-only")
            echo "  1. Extract the ZIP archive" >> "$info_file"
            echo "  2. Copy Chronicle.app to Applications folder" >> "$info_file"
            ;;
        "cli-only")
            echo "  1. Extract the ZIP archive" >> "$info_file"
            echo "  2. Run ./install.sh for automated installation" >> "$info_file"
            echo "  3. Or manually copy bin/* to /usr/local/bin/" >> "$info_file"
            ;;
        "dev")
            echo "  1. Extract the ZIP archive" >> "$info_file"
            echo "  2. Run ./dev_setup.sh if sources are included" >> "$info_file"
            echo "  3. See README.md for development instructions" >> "$info_file"
            ;;
    esac
    
    log "Distribution information saved to: $info_file"
}

# Prepare distribution based on type
prepare_distribution() {
    local staging_dir="$OUTPUT_DIR/staging"
    local dist_name="$APP_NAME-$VERSION-$DISTRIBUTION_TYPE"
    local dist_dir="$staging_dir/$dist_name"
    
    # Clean and create staging directory
    rm -rf "$staging_dir"
    mkdir -p "$dist_dir"
    
    case $DISTRIBUTION_TYPE in
        "full")
            prepare_full_distribution "$dist_dir"
            ;;
        "app-only")
            prepare_app_only_distribution "$dist_dir"
            ;;
        "cli-only")
            prepare_cli_only_distribution "$dist_dir"
            ;;
        "dev")
            prepare_dev_distribution "$dist_dir"
            ;;
    esac
    
    echo "$dist_dir"
}

# Main ZIP creation function
main() {
    log "Starting Chronicle ZIP distribution creation..."
    
    parse_args "$@"
    
    info "ZIP distribution configuration:"
    info "  App Name: $APP_NAME"
    info "  Version: $VERSION"
    info "  Distribution Type: $DISTRIBUTION_TYPE"
    info "  Build Directory: $BUILD_DIR"
    info "  Output Directory: $OUTPUT_DIR"
    info "  Include Sources: $INCLUDE_SOURCES"
    info "  Compression Level: $COMPRESSION_LEVEL"
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    check_zip_prerequisites
    
    local dist_dir
    dist_dir=$(prepare_distribution)
    
    local zip_name="$APP_NAME-$VERSION-$DISTRIBUTION_TYPE.zip"
    create_zip_archive "$dist_dir" "$zip_name"
    generate_distribution_info "$zip_name"
    
    # Cleanup staging
    rm -rf "$(dirname "$dist_dir")"
    
    log "ZIP distribution creation completed successfully!"
    info "ZIP file: $OUTPUT_DIR/$zip_name"
    
    # Open output directory
    if command -v open &> /dev/null; then
        open "$OUTPUT_DIR"
    fi
}

# Run main function
main "$@"