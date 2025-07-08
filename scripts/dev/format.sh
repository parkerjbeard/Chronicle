#!/bin/bash

# Chronicle Development Format Script
# Formats code in all Chronicle components

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
CHECK_ONLY=false
COMPONENTS=()
AUTO_FIX=true

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
Usage: $0 [OPTIONS] [COMPONENTS...]

Format code in Chronicle components.

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -c, --check         Check formatting only (don't modify files)
    --no-fix            Don't automatically fix issues
    --rust              Format only Rust code
    --swift             Format only Swift code
    --shell             Format only shell scripts
    --config            Format only configuration files
    --docs              Format only documentation
    --all               Format all components and file types

COMPONENTS:
    If no components are specified, all components will be formatted.
    Available components: rust, swift, shell, config, docs

EXAMPLES:
    $0                  # Format all code
    $0 --check          # Check formatting without changes
    $0 --rust           # Format only Rust code
    $0 --swift          # Format only Swift code

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
            -c|--check)
                CHECK_ONLY=true
                shift
                ;;
            --no-fix)
                AUTO_FIX=false
                shift
                ;;
            --rust)
                COMPONENTS+=("rust")
                shift
                ;;
            --swift)
                COMPONENTS+=("swift")
                shift
                ;;
            --shell)
                COMPONENTS+=("shell")
                shift
                ;;
            --config)
                COMPONENTS+=("config")
                shift
                ;;
            --docs)
                COMPONENTS+=("docs")
                shift
                ;;
            --all)
                COMPONENTS=("rust" "swift" "shell" "config" "docs")
                shift
                ;;
            -*)
                error "Unknown option: $1"
                ;;
            *)
                COMPONENTS+=("$1")
                shift
                ;;
        esac
    done
    
    # If no components specified, format all
    if [ ${#COMPONENTS[@]} -eq 0 ]; then
        COMPONENTS=("rust" "swift" "shell" "config" "docs")
    fi
}

# Check if tool is available
check_tool() {
    local tool=$1
    local install_hint=$2
    
    if ! command -v "$tool" &> /dev/null; then
        warn "$tool not found. Install with: $install_hint"
        return 1
    fi
    return 0
}

# Format Rust code
format_rust() {
    log "Formatting Rust code..."
    
    cd "$ROOT_DIR"
    
    # Check for rustfmt
    if ! check_tool "rustfmt" "rustup component add rustfmt"; then
        return 1
    fi
    
    # Find all Rust projects
    local rust_projects=()
    for dir in "packer" "cli" "benchmarks" "tests"; do
        if [ -d "$dir" ] && [ -f "$dir/Cargo.toml" ]; then
            rust_projects+=("$dir")
        fi
    done
    
    if [ ${#rust_projects[@]} -eq 0 ]; then
        warn "No Rust projects found"
        return 0
    fi
    
    local format_failed=false
    
    for project in "${rust_projects[@]}"; do
        info "Formatting Rust project: $project"
        cd "$ROOT_DIR/$project"
        
        local rustfmt_flags=""
        if [ "$CHECK_ONLY" = true ]; then
            rustfmt_flags="--check"
        fi
        
        if [ "$VERBOSE" = true ]; then
            rustfmt_flags="$rustfmt_flags --verbose"
        fi
        
        if ! cargo fmt $rustfmt_flags; then
            if [ "$CHECK_ONLY" = true ]; then
                error "Rust code in $project is not formatted correctly"
            else
                warn "Failed to format Rust code in $project"
                format_failed=true
            fi
        fi
        
        # Run clippy for additional linting
        if [ "$AUTO_FIX" = true ] && [ "$CHECK_ONLY" = false ]; then
            info "Running clippy for $project..."
            if ! cargo clippy --fix --allow-dirty --allow-staged; then
                warn "Clippy auto-fix failed for $project"
            fi
        fi
        
        cd "$ROOT_DIR"
    done
    
    if [ "$format_failed" = true ]; then
        error "Some Rust formatting operations failed"
    fi
    
    log "Rust formatting complete"
}

# Format Swift code
format_swift() {
    log "Formatting Swift code..."
    
    cd "$ROOT_DIR"
    
    # Check for swiftformat
    if ! check_tool "swiftformat" "brew install swiftformat"; then
        # Try to use built-in swift-format if available
        if ! check_tool "swift-format" "Install swift-format"; then
            warn "No Swift formatter available, skipping Swift formatting"
            return 0
        fi
    fi
    
    # Find all Swift files
    local swift_files=()
    while IFS= read -r -d '' file; do
        swift_files+=("$file")
    done < <(find . -name "*.swift" -not -path "./build/*" -not -path "./.build/*" -print0)
    
    if [ ${#swift_files[@]} -eq 0 ]; then
        warn "No Swift files found"
        return 0
    fi
    
    info "Found ${#swift_files[@]} Swift files"
    
    local formatter_flags=""
    if [ "$CHECK_ONLY" = true ]; then
        formatter_flags="--lint"
    fi
    
    if [ "$VERBOSE" = true ]; then
        formatter_flags="$formatter_flags --verbose"
    fi
    
    # Use swiftformat if available, otherwise swift-format
    if command -v swiftformat &> /dev/null; then
        info "Using swiftformat..."
        if ! swiftformat $formatter_flags "${swift_files[@]}"; then
            if [ "$CHECK_ONLY" = true ]; then
                error "Swift code is not formatted correctly"
            else
                error "Failed to format Swift code"
            fi
        fi
    else
        info "Using swift-format..."
        for file in "${swift_files[@]}"; do
            if [ "$CHECK_ONLY" = true ]; then
                if ! swift-format lint "$file"; then
                    error "Swift file $file is not formatted correctly"
                fi
            else
                if ! swift-format --in-place "$file"; then
                    error "Failed to format Swift file $file"
                fi
            fi
        done
    fi
    
    log "Swift formatting complete"
}

# Format shell scripts
format_shell() {
    log "Formatting shell scripts..."
    
    cd "$ROOT_DIR"
    
    # Check for shfmt
    if ! check_tool "shfmt" "brew install shfmt"; then
        warn "shfmt not available, skipping shell formatting"
        return 0
    fi
    
    # Find all shell scripts
    local shell_files=()
    while IFS= read -r -d '' file; do
        shell_files+=("$file")
    done < <(find . -name "*.sh" -not -path "./build/*" -not -path "./.build/*" -print0)
    
    if [ ${#shell_files[@]} -eq 0 ]; then
        warn "No shell scripts found"
        return 0
    fi
    
    info "Found ${#shell_files[@]} shell scripts"
    
    local shfmt_flags="-i 4 -bn -ci"  # 4 spaces, binary ops at beginning, case indentation
    if [ "$CHECK_ONLY" = true ]; then
        shfmt_flags="$shfmt_flags -d"
    else
        shfmt_flags="$shfmt_flags -w"
    fi
    
    if ! shfmt $shfmt_flags "${shell_files[@]}"; then
        if [ "$CHECK_ONLY" = true ]; then
            error "Shell scripts are not formatted correctly"
        else
            error "Failed to format shell scripts"
        fi
    fi
    
    # Check shell scripts with shellcheck
    if check_tool "shellcheck" "brew install shellcheck"; then
        info "Running shellcheck..."
        local shellcheck_failed=false
        for file in "${shell_files[@]}"; do
            if ! shellcheck "$file"; then
                shellcheck_failed=true
            fi
        done
        
        if [ "$shellcheck_failed" = true ]; then
            warn "Some shell scripts have shellcheck warnings"
        fi
    fi
    
    log "Shell formatting complete"
}

# Format configuration files
format_config() {
    log "Formatting configuration files..."
    
    cd "$ROOT_DIR"
    
    # Format TOML files
    if check_tool "taplo" "cargo install taplo-cli"; then
        local toml_files=()
        while IFS= read -r -d '' file; do
            toml_files+=("$file")
        done < <(find . -name "*.toml" -not -path "./target/*" -not -path "./build/*" -print0)
        
        if [ ${#toml_files[@]} -gt 0 ]; then
            info "Found ${#toml_files[@]} TOML files"
            
            local taplo_flags=""
            if [ "$CHECK_ONLY" = true ]; then
                taplo_flags="check"
            else
                taplo_flags="format"
            fi
            
            if ! taplo $taplo_flags "${toml_files[@]}"; then
                if [ "$CHECK_ONLY" = true ]; then
                    error "TOML files are not formatted correctly"
                else
                    error "Failed to format TOML files"
                fi
            fi
        fi
    fi
    
    # Format JSON files
    if check_tool "jq" "brew install jq"; then
        local json_files=()
        while IFS= read -r -d '' file; do
            json_files+=("$file")
        done < <(find . -name "*.json" -not -path "./target/*" -not -path "./build/*" -print0)
        
        if [ ${#json_files[@]} -gt 0 ]; then
            info "Found ${#json_files[@]} JSON files"
            
            if [ "$CHECK_ONLY" = false ]; then
                for file in "${json_files[@]}"; do
                    if ! jq --indent 4 . "$file" > "$file.tmp" && mv "$file.tmp" "$file"; then
                        error "Failed to format JSON file $file"
                    fi
                done
            else
                for file in "${json_files[@]}"; do
                    if ! jq . "$file" > /dev/null; then
                        error "JSON file $file is not valid"
                    fi
                done
            fi
        fi
    fi
    
    log "Configuration formatting complete"
}

# Format documentation
format_docs() {
    log "Formatting documentation..."
    
    cd "$ROOT_DIR"
    
    # Format Markdown files
    if check_tool "markdownlint" "npm install -g markdownlint-cli"; then
        local md_files=()
        while IFS= read -r -d '' file; do
            md_files+=("$file")
        done < <(find . -name "*.md" -not -path "./target/*" -not -path "./build/*" -print0)
        
        if [ ${#md_files[@]} -gt 0 ]; then
            info "Found ${#md_files[@]} Markdown files"
            
            local markdownlint_flags=""
            if [ "$CHECK_ONLY" = false ] && [ "$AUTO_FIX" = true ]; then
                markdownlint_flags="--fix"
            fi
            
            if ! markdownlint $markdownlint_flags "${md_files[@]}"; then
                if [ "$CHECK_ONLY" = true ]; then
                    error "Markdown files have formatting issues"
                else
                    warn "Some Markdown files have formatting issues"
                fi
            fi
        fi
    fi
    
    log "Documentation formatting complete"
}

# Format a single component
format_component() {
    local component=$1
    
    case $component in
        "rust")
            format_rust
            ;;
        "swift")
            format_swift
            ;;
        "shell")
            format_shell
            ;;
        "config")
            format_config
            ;;
        "docs")
            format_docs
            ;;
        *)
            error "Unknown component: $component"
            ;;
    esac
}

# Format all components
format_all() {
    log "Formatting components: ${COMPONENTS[*]}"
    
    local format_failed=false
    
    for component in "${COMPONENTS[@]}"; do
        if ! format_component "$component"; then
            format_failed=true
        fi
    done
    
    if [ "$format_failed" = true ]; then
        error "Some formatting operations failed"
    fi
}

# Main format function
main() {
    log "Starting Chronicle code formatting..."
    
    parse_args "$@"
    
    info "Format configuration:"
    info "  Components: ${COMPONENTS[*]}"
    info "  Check only: $CHECK_ONLY"
    info "  Auto fix: $AUTO_FIX"
    info "  Verbose: $VERBOSE"
    
    cd "$ROOT_DIR"
    
    format_all
    
    log "Code formatting complete!"
    
    if [ "$CHECK_ONLY" = true ]; then
        info "All code is properly formatted"
    else
        info "Code has been formatted successfully"
    fi
}

# Run main function
main "$@"