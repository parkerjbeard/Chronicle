#!/bin/bash

# Chronicle Package Validation Script
# Validates different package formats (DMG, PKG, ZIP)

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
PACKAGE_PATH=""
PACKAGE_TYPE=""
OUTPUT_DIR=""
DEEP_VALIDATION=false

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
Usage: $0 [OPTIONS] PACKAGE_PATH

Validate Chronicle package files.

ARGUMENTS:
    PACKAGE_PATH            Path to package file (.dmg, .pkg, .zip)

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    --type TYPE             Package type (dmg, pkg, zip) - auto-detected if not specified
    --output-dir DIR        Output directory for validation report
    --deep                  Perform deep validation (extract and test contents)

EXAMPLES:
    $0 Chronicle-1.0.0.dmg
    $0 --deep Chronicle-1.0.0.pkg
    $0 --type zip Chronicle-1.0.0-full.zip

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
            --type)
                PACKAGE_TYPE="$2"
                shift 2
                ;;
            --output-dir)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            --deep)
                DEEP_VALIDATION=true
                shift
                ;;
            -*)
                error "Unknown option: $1"
                ;;
            *)
                if [ -z "$PACKAGE_PATH" ]; then
                    PACKAGE_PATH="$1"
                else
                    error "Multiple package paths specified"
                fi
                shift
                ;;
        esac
    done
    
    # Validate required parameters
    if [ -z "$PACKAGE_PATH" ]; then
        error "Package path is required"
    fi
    
    # Make package path absolute
    if [[ ! "$PACKAGE_PATH" = /* ]]; then
        PACKAGE_PATH="$(pwd)/$PACKAGE_PATH"
    fi
    
    # Auto-detect package type if not specified
    if [ -z "$PACKAGE_TYPE" ]; then
        case "${PACKAGE_PATH##*.}" in
            dmg)
                PACKAGE_TYPE="dmg"
                ;;
            pkg)
                PACKAGE_TYPE="pkg"
                ;;
            zip)
                PACKAGE_TYPE="zip"
                ;;
            *)
                error "Cannot auto-detect package type. Use --type to specify."
                ;;
        esac
    fi
    
    # Set default output directory
    if [ -z "$OUTPUT_DIR" ]; then
        OUTPUT_DIR="$(dirname "$PACKAGE_PATH")/validation"
    fi
    
    # Make output directory absolute
    if [[ ! "$OUTPUT_DIR" = /* ]]; then
        OUTPUT_DIR="$(pwd)/$OUTPUT_DIR"
    fi
}

# Check validation prerequisites
check_validation_prerequisites() {
    log "Checking validation prerequisites..."
    
    # Check if package exists
    if [ ! -f "$PACKAGE_PATH" ]; then
        error "Package file not found: $PACKAGE_PATH"
    fi
    
    # Check for required tools based on package type
    case $PACKAGE_TYPE in
        "dmg")
            if ! command -v hdiutil &> /dev/null; then
                error "hdiutil not found (required for DMG validation)"
            fi
            ;;
        "pkg")
            if ! command -v pkgutil &> /dev/null; then
                error "pkgutil not found (required for PKG validation)"
            fi
            ;;
        "zip")
            if ! command -v unzip &> /dev/null; then
                error "unzip not found (required for ZIP validation)"
            fi
            ;;
    esac
    
    log "Prerequisites check completed"
}

# Basic file validation
validate_basic_file_info() {
    log "Validating basic file information..."
    
    local validation_log="$OUTPUT_DIR/basic_validation.txt"
    
    cat > "$validation_log" << EOF
Chronicle Package Basic Validation
Package: $PACKAGE_PATH
Type: $PACKAGE_TYPE
Generated: $(date)

File Information:
  Size: $(ls -lh "$PACKAGE_PATH" | awk '{print $5}')
  Permissions: $(ls -l "$PACKAGE_PATH" | awk '{print $1}')
  Modified: $(ls -l "$PACKAGE_PATH" | awk '{print $6, $7, $8}')
  
Checksums:
  SHA256: $(shasum -a 256 "$PACKAGE_PATH" | awk '{print $1}')
  MD5: $(md5 -q "$PACKAGE_PATH")

EOF
    
    # File type detection
    echo "File Type:" >> "$validation_log"
    file "$PACKAGE_PATH" >> "$validation_log"
    
    log "Basic file validation completed"
}

# Validate DMG package
validate_dmg_package() {
    log "Validating DMG package..."
    
    local dmg_log="$OUTPUT_DIR/dmg_validation.txt"
    
    cat > "$dmg_log" << EOF
Chronicle DMG Validation
Package: $PACKAGE_PATH
Generated: $(date)

DMG Verification:
EOF
    
    # Verify DMG integrity
    if hdiutil verify "$PACKAGE_PATH" >> "$dmg_log" 2>&1; then
        echo "  Integrity: PASSED" >> "$dmg_log"
        info "DMG integrity check passed"
    else
        echo "  Integrity: FAILED" >> "$dmg_log"
        warn "DMG integrity check failed"
    fi
    
    # Get DMG information
    echo "" >> "$dmg_log"
    echo "DMG Information:" >> "$dmg_log"
    hdiutil imageinfo "$PACKAGE_PATH" >> "$dmg_log" 2>&1 || true
    
    # Test mount/unmount if deep validation
    if [ "$DEEP_VALIDATION" = true ]; then
        log "Performing deep DMG validation..."
        
        local mount_point="/tmp/chronicle_dmg_validation_$$"
        mkdir -p "$mount_point"
        
        echo "" >> "$dmg_log"
        echo "Mount Test:" >> "$dmg_log"
        
        if hdiutil attach "$PACKAGE_PATH" -mountpoint "$mount_point" -readonly -nobrowse >> "$dmg_log" 2>&1; then
            echo "  Mount: PASSED" >> "$dmg_log"
            
            # List contents
            echo "" >> "$dmg_log"
            echo "Contents:" >> "$dmg_log"
            ls -la "$mount_point" >> "$dmg_log" 2>&1 || true
            
            # Validate expected contents
            validate_dmg_contents "$mount_point" "$dmg_log"
            
            # Unmount
            if hdiutil detach "$mount_point" >> "$dmg_log" 2>&1; then
                echo "  Unmount: PASSED" >> "$dmg_log"
            else
                echo "  Unmount: FAILED" >> "$dmg_log"
                warn "Failed to unmount DMG"
            fi
        else
            echo "  Mount: FAILED" >> "$dmg_log"
            warn "Failed to mount DMG"
        fi
        
        rmdir "$mount_point" 2>/dev/null || true
    fi
    
    log "DMG validation completed"
}

# Validate DMG contents
validate_dmg_contents() {
    local mount_point="$1"
    local dmg_log="$2"
    
    echo "" >> "$dmg_log"
    echo "Content Validation:" >> "$dmg_log"
    
    # Check for Chronicle app
    if [ -d "$mount_point/Chronicle.app" ]; then
        echo "  Chronicle.app: FOUND" >> "$dmg_log"
        
        # Validate app bundle structure
        if [ -f "$mount_point/Chronicle.app/Contents/Info.plist" ]; then
            echo "    Info.plist: FOUND" >> "$dmg_log"
        else
            echo "    Info.plist: MISSING" >> "$dmg_log"
        fi
        
        if [ -f "$mount_point/Chronicle.app/Contents/MacOS/ChronicleUI" ]; then
            echo "    Executable: FOUND" >> "$dmg_log"
        else
            echo "    Executable: MISSING" >> "$dmg_log"
        fi
    else
        echo "  Chronicle.app: MISSING" >> "$dmg_log"
    fi
    
    # Check for Applications symlink
    if [ -L "$mount_point/Applications" ]; then
        echo "  Applications symlink: FOUND" >> "$dmg_log"
    else
        echo "  Applications symlink: MISSING" >> "$dmg_log"
    fi
    
    # Check for CLI tools
    if [ -d "$mount_point/Command Line Tools" ]; then
        echo "  Command Line Tools: FOUND" >> "$dmg_log"
        ls -la "$mount_point/Command Line Tools" >> "$dmg_log" 2>&1 || true
    else
        echo "  Command Line Tools: MISSING" >> "$dmg_log"
    fi
    
    # Check for documentation
    if [ -d "$mount_point/Documentation" ]; then
        echo "  Documentation: FOUND" >> "$dmg_log"
    else
        echo "  Documentation: MISSING" >> "$dmg_log"
    fi
}

# Validate PKG package
validate_pkg_package() {
    log "Validating PKG package..."
    
    local pkg_log="$OUTPUT_DIR/pkg_validation.txt"
    
    cat > "$pkg_log" << EOF
Chronicle PKG Validation
Package: $PACKAGE_PATH
Generated: $(date)

PKG Information:
EOF
    
    # Get PKG information
    pkgutil --pkg-info-plist "$PACKAGE_PATH" >> "$pkg_log" 2>&1 || true
    
    echo "" >> "$pkg_log"
    echo "Signature Check:" >> "$pkg_log"
    
    # Check signature
    if pkgutil --check-signature "$PACKAGE_PATH" >> "$pkg_log" 2>&1; then
        echo "  Signature: VALID" >> "$pkg_log"
        info "PKG signature is valid"
    else
        echo "  Signature: INVALID or UNSIGNED" >> "$pkg_log"
        warn "PKG signature check failed"
    fi
    
    echo "" >> "$pkg_log"
    echo "File List:" >> "$pkg_log"
    
    # List files that would be installed
    pkgutil --payload-files "$PACKAGE_PATH" >> "$pkg_log" 2>&1 || true
    
    # Deep validation
    if [ "$DEEP_VALIDATION" = true ]; then
        log "Performing deep PKG validation..."
        
        local extract_dir="$OUTPUT_DIR/pkg_extract"
        mkdir -p "$extract_dir"
        
        echo "" >> "$pkg_log"
        echo "Extraction Test:" >> "$pkg_log"
        
        # Extract payload
        if pkgutil --expand "$PACKAGE_PATH" "$extract_dir" >> "$pkg_log" 2>&1; then
            echo "  Extraction: PASSED" >> "$pkg_log"
            
            # Validate extracted contents
            validate_pkg_contents "$extract_dir" "$pkg_log"
        else
            echo "  Extraction: FAILED" >> "$pkg_log"
            warn "Failed to extract PKG contents"
        fi
    fi
    
    log "PKG validation completed"
}

# Validate PKG contents
validate_pkg_contents() {
    local extract_dir="$1"
    local pkg_log="$2"
    
    echo "" >> "$pkg_log"
    echo "Extracted Content Validation:" >> "$pkg_log"
    
    # List extracted components
    find "$extract_dir" -name "*.pkg" >> "$pkg_log" 2>&1 || true
    
    # Check for distribution file
    if [ -f "$extract_dir/Distribution" ]; then
        echo "  Distribution file: FOUND" >> "$pkg_log"
    else
        echo "  Distribution file: MISSING" >> "$pkg_log"
    fi
    
    # Check for resources
    if [ -d "$extract_dir/Resources" ]; then
        echo "  Resources: FOUND" >> "$pkg_log"
        ls -la "$extract_dir/Resources" >> "$pkg_log" 2>&1 || true
    else
        echo "  Resources: MISSING" >> "$pkg_log"
    fi
}

# Validate ZIP package
validate_zip_package() {
    log "Validating ZIP package..."
    
    local zip_log="$OUTPUT_DIR/zip_validation.txt"
    
    cat > "$zip_log" << EOF
Chronicle ZIP Validation
Package: $PACKAGE_PATH
Generated: $(date)

ZIP Information:
EOF
    
    # Test ZIP integrity
    if unzip -t "$PACKAGE_PATH" >> "$zip_log" 2>&1; then
        echo "  Integrity: PASSED" >> "$zip_log"
        info "ZIP integrity check passed"
    else
        echo "  Integrity: FAILED" >> "$zip_log"
        warn "ZIP integrity check failed"
    fi
    
    echo "" >> "$zip_log"
    echo "File List:" >> "$zip_log"
    
    # List ZIP contents
    unzip -l "$PACKAGE_PATH" >> "$zip_log" 2>&1 || true
    
    # Deep validation
    if [ "$DEEP_VALIDATION" = true ]; then
        log "Performing deep ZIP validation..."
        
        local extract_dir="$OUTPUT_DIR/zip_extract"
        mkdir -p "$extract_dir"
        
        echo "" >> "$zip_log"
        echo "Extraction Test:" >> "$zip_log"
        
        # Extract ZIP
        if unzip -q "$PACKAGE_PATH" -d "$extract_dir" >> "$zip_log" 2>&1; then
            echo "  Extraction: PASSED" >> "$zip_log"
            
            # Validate extracted contents
            validate_zip_contents "$extract_dir" "$zip_log"
        else
            echo "  Extraction: FAILED" >> "$zip_log"
            warn "Failed to extract ZIP contents"
        fi
    fi
    
    log "ZIP validation completed"
}

# Validate ZIP contents
validate_zip_contents() {
    local extract_dir="$1"
    local zip_log="$2"
    
    echo "" >> "$zip_log"
    echo "Extracted Content Validation:" >> "$zip_log"
    
    # Find the main directory
    local main_dir=$(find "$extract_dir" -maxdepth 1 -type d -name "Chronicle-*" | head -1)
    
    if [ -n "$main_dir" ]; then
        echo "  Main directory: $(basename "$main_dir")" >> "$zip_log"
        
        # Check for app bundle
        if [ -d "$main_dir/Chronicle.app" ]; then
            echo "  Chronicle.app: FOUND" >> "$zip_log"
        else
            echo "  Chronicle.app: NOT FOUND" >> "$zip_log"
        fi
        
        # Check for CLI tools
        if [ -d "$main_dir/bin" ]; then
            echo "  CLI tools: FOUND" >> "$zip_log"
            ls -la "$main_dir/bin" >> "$zip_log" 2>&1 || true
        else
            echo "  CLI tools: NOT FOUND" >> "$zip_log"
        fi
        
        # Check for installation script
        if [ -f "$main_dir/install.sh" ]; then
            echo "  Installation script: FOUND" >> "$zip_log"
        else
            echo "  Installation script: NOT FOUND" >> "$zip_log"
        fi
        
        # Check for documentation
        local docs_found=false
        for doc in "README.md" "LICENSE" "docs"; do
            if [ -e "$main_dir/$doc" ]; then
                docs_found=true
                break
            fi
        done
        
        if [ "$docs_found" = true ]; then
            echo "  Documentation: FOUND" >> "$zip_log"
        else
            echo "  Documentation: NOT FOUND" >> "$zip_log"
        fi
        
    else
        echo "  Main directory: NOT FOUND" >> "$zip_log"
    fi
}

# Validate code signatures
validate_code_signatures() {
    log "Validating code signatures..."
    
    local sig_log="$OUTPUT_DIR/signature_validation.txt"
    
    cat > "$sig_log" << EOF
Chronicle Code Signature Validation
Package: $PACKAGE_PATH
Generated: $(date)

EOF
    
    case $PACKAGE_TYPE in
        "dmg")
            echo "DMG Signature:" >> "$sig_log"
            codesign -dv "$PACKAGE_PATH" >> "$sig_log" 2>&1 || echo "No signature or verification failed" >> "$sig_log"
            ;;
        "pkg")
            echo "PKG Signature:" >> "$sig_log"
            pkgutil --check-signature "$PACKAGE_PATH" >> "$sig_log" 2>&1 || echo "No signature or verification failed" >> "$sig_log"
            ;;
        "zip")
            echo "ZIP files typically don't have signatures" >> "$sig_log"
            ;;
    esac
    
    log "Code signature validation completed"
}

# Generate validation summary
generate_validation_summary() {
    log "Generating validation summary..."
    
    local summary_file="$OUTPUT_DIR/validation_summary.txt"
    
    cat > "$summary_file" << EOF
Chronicle Package Validation Summary
Package: $PACKAGE_PATH
Type: $PACKAGE_TYPE
Validation Date: $(date)
Deep Validation: $DEEP_VALIDATION

Validation Results:
EOF
    
    # Check if validation logs exist and summarize results
    local overall_status="PASSED"
    
    if [ -f "$OUTPUT_DIR/basic_validation.txt" ]; then
        echo "  Basic File Check: COMPLETED" >> "$summary_file"
    fi
    
    case $PACKAGE_TYPE in
        "dmg")
            if [ -f "$OUTPUT_DIR/dmg_validation.txt" ]; then
                echo "  DMG Validation: COMPLETED" >> "$summary_file"
                if grep -q "FAILED" "$OUTPUT_DIR/dmg_validation.txt"; then
                    overall_status="FAILED"
                fi
            fi
            ;;
        "pkg")
            if [ -f "$OUTPUT_DIR/pkg_validation.txt" ]; then
                echo "  PKG Validation: COMPLETED" >> "$summary_file"
                if grep -q "FAILED" "$OUTPUT_DIR/pkg_validation.txt"; then
                    overall_status="FAILED"
                fi
            fi
            ;;
        "zip")
            if [ -f "$OUTPUT_DIR/zip_validation.txt" ]; then
                echo "  ZIP Validation: COMPLETED" >> "$summary_file"
                if grep -q "FAILED" "$OUTPUT_DIR/zip_validation.txt"; then
                    overall_status="FAILED"
                fi
            fi
            ;;
    esac
    
    if [ -f "$OUTPUT_DIR/signature_validation.txt" ]; then
        echo "  Signature Check: COMPLETED" >> "$summary_file"
    fi
    
    echo "" >> "$summary_file"
    echo "Overall Status: $overall_status" >> "$summary_file"
    
    echo "" >> "$summary_file"
    echo "Detailed Reports:" >> "$summary_file"
    for report in "$OUTPUT_DIR"/*.txt; do
        if [ -f "$report" ] && [ "$(basename "$report")" != "validation_summary.txt" ]; then
            echo "  $(basename "$report")" >> "$summary_file"
        fi
    done
    
    log "Validation summary saved to: $summary_file"
    
    if [ "$overall_status" = "PASSED" ]; then
        log "Package validation PASSED"
    else
        warn "Package validation FAILED - check detailed reports"
    fi
}

# Cleanup extracted files
cleanup_validation() {
    log "Cleaning up validation artifacts..."
    
    # Remove extracted directories
    for extract_dir in "$OUTPUT_DIR/pkg_extract" "$OUTPUT_DIR/zip_extract"; do
        if [ -d "$extract_dir" ]; then
            rm -rf "$extract_dir"
        fi
    done
    
    log "Cleanup completed"
}

# Main validation function
main() {
    log "Starting Chronicle package validation..."
    
    parse_args "$@"
    
    info "Package validation configuration:"
    info "  Package Path: $PACKAGE_PATH"
    info "  Package Type: $PACKAGE_TYPE"
    info "  Output Directory: $OUTPUT_DIR"
    info "  Deep Validation: $DEEP_VALIDATION"
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    check_validation_prerequisites
    validate_basic_file_info
    
    case $PACKAGE_TYPE in
        "dmg")
            validate_dmg_package
            ;;
        "pkg")
            validate_pkg_package
            ;;
        "zip")
            validate_zip_package
            ;;
    esac
    
    validate_code_signatures
    generate_validation_summary
    cleanup_validation
    
    log "Package validation completed!"
    info "Validation reports available in: $OUTPUT_DIR"
    
    # Open output directory
    if command -v open &> /dev/null; then
        open "$OUTPUT_DIR"
    fi
}

# Run main function
main "$@"