#!/bin/bash

# Chronicle Development Clean Script
# Cleans build artifacts and temporary files

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
DRY_RUN=false
DEEP_CLEAN=false
COMPONENTS=()

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

Clean Chronicle build artifacts and temporary files.

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -n, --dry-run       Show what would be deleted without deleting
    -d, --deep          Deep clean (includes caches and derived data)
    --ring-buffer       Clean only ring buffer component
    --packer            Clean only packer component
    --collectors        Clean only collectors component
    --cli               Clean only CLI component
    --ui                Clean only UI component
    --benchmarks        Clean only benchmarks component
    --tests             Clean only tests component
    --logs              Clean only log files
    --cache             Clean only cache files
    --all               Clean everything (equivalent to deep clean)

COMPONENTS:
    If no components are specified, all components will be cleaned.
    Available components: ring-buffer, packer, collectors, cli, ui, benchmarks, tests, logs, cache

EXAMPLES:
    $0                  # Clean all build artifacts
    $0 --deep           # Deep clean including caches
    $0 --dry-run        # Show what would be cleaned
    $0 cli packer       # Clean only CLI and packer components

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
            -n|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -d|--deep)
                DEEP_CLEAN=true
                shift
                ;;
            --all)
                DEEP_CLEAN=true
                shift
                ;;
            --ring-buffer)
                COMPONENTS+=("ring-buffer")
                shift
                ;;
            --packer)
                COMPONENTS+=("packer")
                shift
                ;;
            --collectors)
                COMPONENTS+=("collectors")
                shift
                ;;
            --cli)
                COMPONENTS+=("cli")
                shift
                ;;
            --ui)
                COMPONENTS+=("ui")
                shift
                ;;
            --benchmarks)
                COMPONENTS+=("benchmarks")
                shift
                ;;
            --tests)
                COMPONENTS+=("tests")
                shift
                ;;
            --logs)
                COMPONENTS+=("logs")
                shift
                ;;
            --cache)
                COMPONENTS+=("cache")
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
    
    # If no components specified, clean all
    if [ ${#COMPONENTS[@]} -eq 0 ]; then
        COMPONENTS=("ring-buffer" "packer" "collectors" "cli" "ui" "benchmarks" "tests" "logs" "cache")
    fi
}

# Safe remove function
safe_remove() {
    local path="$1"
    local description="$2"
    
    if [ -e "$path" ]; then
        if [ "$DRY_RUN" = true ]; then
            info "Would remove: $path ($description)"
        else
            if [ "$VERBOSE" = true ]; then
                info "Removing: $path ($description)"
            fi
            rm -rf "$path"
        fi
    fi
}

# Calculate directory size
get_size() {
    local path="$1"
    if [ -e "$path" ]; then
        du -sh "$path" 2>/dev/null | cut -f1
    else
        echo "0B"
    fi
}

# Clean ring buffer component
clean_ring_buffer() {
    log "Cleaning ring buffer component..."
    
    cd "$ROOT_DIR/ring-buffer"
    
    # Clean build artifacts
    safe_remove "*.o" "object files"
    safe_remove "*.a" "library files"
    safe_remove "test_ring_buffer" "test executable"
    safe_remove "bench_ring_buffer" "benchmark executable"
    
    # Run make clean if available
    if [ -f "Makefile" ] && [ "$DRY_RUN" = false ]; then
        make clean 2>/dev/null || true
    fi
    
    log "Ring buffer clean complete"
}

# Clean Rust components
clean_rust_component() {
    local component=$1
    local component_dir="$ROOT_DIR/$component"
    
    log "Cleaning Rust component: $component..."
    
    if [ ! -d "$component_dir" ]; then
        warn "Component directory not found: $component_dir"
        return
    fi
    
    cd "$component_dir"
    
    # Show size before cleaning
    local size_before=$(get_size "target")
    if [ "$VERBOSE" = true ] && [ "$size_before" != "0B" ]; then
        info "Target directory size: $size_before"
    fi
    
    # Clean target directory
    safe_remove "target" "Rust build artifacts"
    
    # Clean Cargo.lock if it exists (for workspace members)
    if [ -f "Cargo.lock" ] && [ "$component" != "cli" ]; then
        safe_remove "Cargo.lock" "Cargo lock file"
    fi
    
    # Clean coverage reports
    safe_remove "coverage" "coverage reports"
    
    # Clean profiling data
    safe_remove "*.profraw" "profiling data"
    
    log "$component clean complete"
}

# Clean Swift components
clean_swift_component() {
    local component=$1
    
    log "Cleaning Swift component: $component..."
    
    cd "$ROOT_DIR"
    
    # Clean Xcode build products
    safe_remove "build" "Xcode build products"
    
    # Clean derived data
    local derived_data_path="$HOME/Library/Developer/Xcode/DerivedData"
    if [ -d "$derived_data_path" ]; then
        for dd_dir in "$derived_data_path"/Chronicle-*; do
            if [ -d "$dd_dir" ]; then
                safe_remove "$dd_dir" "Xcode derived data"
            fi
        done
    fi
    
    # Clean module cache
    safe_remove "$HOME/Library/Developer/Xcode/DerivedData/ModuleCache.noindex" "module cache"
    
    log "$component clean complete"
}

# Clean logs
clean_logs() {
    log "Cleaning log files..."
    
    cd "$ROOT_DIR"
    
    # Clean log directories
    safe_remove "logs" "log files"
    safe_remove "*.log" "log files"
    
    # Clean crash logs
    safe_remove "crash_*.log" "crash logs"
    
    # Clean system logs (if any)
    if [ -d "/tmp/chronicle_logs" ]; then
        safe_remove "/tmp/chronicle_logs" "temporary log files"
    fi
    
    log "Log clean complete"
}

# Clean cache files
clean_cache() {
    log "Cleaning cache files..."
    
    cd "$ROOT_DIR"
    
    # Clean temp directories
    safe_remove "temp" "temporary files"
    safe_remove "tmp" "temporary files"
    safe_remove ".tmp" "temporary files"
    
    # Clean cache directories
    safe_remove "cache" "cache files"
    safe_remove ".cache" "cache files"
    
    # Clean Python cache
    find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
    find . -name "*.pyc" -type f -delete 2>/dev/null || true
    
    # Clean Node.js cache (if any)
    safe_remove "node_modules" "Node.js modules"
    safe_remove "package-lock.json" "Node.js lock file"
    
    log "Cache clean complete"
}

# Deep clean
deep_clean() {
    log "Performing deep clean..."
    
    cd "$ROOT_DIR"
    
    # Clean Rust global cache
    if [ "$DRY_RUN" = false ]; then
        cargo clean 2>/dev/null || true
    fi
    
    # Clean Homebrew cache
    if command -v brew &> /dev/null; then
        if [ "$DRY_RUN" = false ]; then
            brew cleanup --prune=all 2>/dev/null || true
        else
            info "Would clean Homebrew cache"
        fi
    fi
    
    # Clean system caches
    if [ -d "$HOME/Library/Caches/com.chronicle" ]; then
        safe_remove "$HOME/Library/Caches/com.chronicle" "application cache"
    fi
    
    # Clean development tools cache
    safe_remove "$HOME/.cargo/registry/cache" "Cargo registry cache"
    safe_remove "$HOME/.rustup/toolchains/*/share/doc" "Rust documentation"
    
    log "Deep clean complete"
}

# Clean a single component
clean_component() {
    local component=$1
    
    case $component in
        "ring-buffer")
            clean_ring_buffer
            ;;
        "packer")
            clean_rust_component "packer"
            ;;
        "cli")
            clean_rust_component "cli"
            ;;
        "benchmarks")
            clean_rust_component "benchmarks"
            ;;
        "tests")
            clean_rust_component "tests"
            ;;
        "collectors")
            clean_swift_component "collectors"
            ;;
        "ui")
            clean_swift_component "ui"
            ;;
        "logs")
            clean_logs
            ;;
        "cache")
            clean_cache
            ;;
        *)
            error "Unknown component: $component"
            ;;
    esac
}

# Clean all components
clean_all() {
    log "Cleaning components: ${COMPONENTS[*]}"
    
    for component in "${COMPONENTS[@]}"; do
        clean_component "$component"
    done
    
    if [ "$DEEP_CLEAN" = true ]; then
        deep_clean
    fi
}

# Calculate total space saved
calculate_savings() {
    log "Calculating space savings..."
    
    # This is a simplified calculation
    # In practice, you'd want to measure before and after
    local total_saved="0"
    
    info "Clean operation complete"
    if [ "$DRY_RUN" = true ]; then
        info "This was a dry run - no files were actually deleted"
    fi
}

# Main clean function
main() {
    log "Starting Chronicle clean..."
    
    parse_args "$@"
    
    info "Clean configuration:"
    info "  Components: ${COMPONENTS[*]}"
    info "  Deep clean: $DEEP_CLEAN"
    info "  Verbose: $VERBOSE"
    info "  Dry run: $DRY_RUN"
    
    cd "$ROOT_DIR"
    
    clean_all
    calculate_savings
    
    log "Clean complete!"
    
    if [ "$DRY_RUN" = true ]; then
        info "Run without --dry-run to actually delete files"
    fi
}

# Run main function
main "$@"