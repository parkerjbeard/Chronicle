#!/bin/bash

# Chronicle Development Build Script
# Builds all Chronicle components for development

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
BUILD_TYPE="debug"
VERBOSE=false
CLEAN=false
COMPONENTS=()
PARALLEL=true

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

Build Chronicle components for development.

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -c, --clean         Clean before building
    -r, --release       Build in release mode
    -s, --serial        Build components serially (not in parallel)
    --ring-buffer       Build only ring buffer component
    --packer            Build only packer component
    --collectors        Build only collectors component
    --cli               Build only CLI component
    --ui                Build only UI component
    --benchmarks        Build only benchmarks component
    --tests             Build only tests component

COMPONENTS:
    If no components are specified, all components will be built.
    Available components: ring-buffer, packer, collectors, cli, ui, benchmarks, tests

EXAMPLES:
    $0                  # Build all components in debug mode
    $0 --release        # Build all components in release mode
    $0 --clean cli ui   # Clean and build CLI and UI components
    $0 --verbose packer # Build packer with verbose output

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
            -c|--clean)
                CLEAN=true
                shift
                ;;
            -r|--release)
                BUILD_TYPE="release"
                shift
                ;;
            -s|--serial)
                PARALLEL=false
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
            -*)
                error "Unknown option: $1"
                ;;
            *)
                COMPONENTS+=("$1")
                shift
                ;;
        esac
    done
    
    # If no components specified, build all
    if [ ${#COMPONENTS[@]} -eq 0 ]; then
        COMPONENTS=("ring-buffer" "packer" "collectors" "cli" "ui" "benchmarks" "tests")
    fi
}

# Clean build artifacts
clean_build() {
    if [ "$CLEAN" = true ]; then
        log "Cleaning build artifacts..."
        
        cd "$ROOT_DIR"
        
        # Clean Rust targets
        if [ -d "target" ]; then
            rm -rf target
        fi
        
        # Clean Xcode build products
        if [ -d "build" ]; then
            rm -rf build
        fi
        
        # Clean derived data
        if [ -d ~/Library/Developer/Xcode/DerivedData ]; then
            rm -rf ~/Library/Developer/Xcode/DerivedData/Chronicle-*
        fi
        
        # Clean ring buffer artifacts
        if [ -d "ring-buffer" ]; then
            cd ring-buffer
            make clean 2>/dev/null || true
            cd ..
        fi
        
        log "Clean complete"
    fi
}

# Build ring buffer component
build_ring_buffer() {
    log "Building ring buffer component..."
    
    cd "$ROOT_DIR/ring-buffer"
    
    local make_flags=""
    if [ "$VERBOSE" = true ]; then
        make_flags="VERBOSE=1"
    fi
    
    if [ "$BUILD_TYPE" = "release" ]; then
        make_flags="$make_flags RELEASE=1"
    fi
    
    if ! make $make_flags; then
        error "Ring buffer build failed"
    fi
    
    log "Ring buffer build complete"
}

# Build Rust components
build_rust_component() {
    local component=$1
    local component_dir="$ROOT_DIR/$component"
    
    log "Building Rust component: $component..."
    
    if [ ! -d "$component_dir" ]; then
        error "Component directory not found: $component_dir"
    fi
    
    cd "$component_dir"
    
    local cargo_flags=""
    if [ "$VERBOSE" = true ]; then
        cargo_flags="--verbose"
    fi
    
    if [ "$BUILD_TYPE" = "release" ]; then
        cargo_flags="$cargo_flags --release"
    fi
    
    if ! cargo build $cargo_flags; then
        error "$component build failed"
    fi
    
    log "$component build complete"
}

# Build Swift components
build_swift_component() {
    local component=$1
    local workspace_scheme=""
    
    log "Building Swift component: $component..."
    
    cd "$ROOT_DIR"
    
    case $component in
        "collectors")
            workspace_scheme="ChronicleCollectors"
            ;;
        "ui")
            workspace_scheme="ChronicleUI"
            ;;
        *)
            error "Unknown Swift component: $component"
            ;;
    esac
    
    local xcode_flags=""
    if [ "$VERBOSE" = false ]; then
        xcode_flags="-quiet"
    fi
    
    local configuration="Debug"
    if [ "$BUILD_TYPE" = "release" ]; then
        configuration="Release"
    fi
    
    if ! xcodebuild -workspace Chronicle.xcworkspace \
                    -scheme "$workspace_scheme" \
                    -configuration "$configuration" \
                    -derivedDataPath build \
                    $xcode_flags \
                    build; then
        error "$component build failed"
    fi
    
    log "$component build complete"
}

# Build a single component
build_component() {
    local component=$1
    
    case $component in
        "ring-buffer")
            build_ring_buffer
            ;;
        "packer")
            build_rust_component "packer"
            ;;
        "cli")
            build_rust_component "cli"
            ;;
        "benchmarks")
            build_rust_component "benchmarks"
            ;;
        "tests")
            build_rust_component "tests"
            ;;
        "collectors")
            build_swift_component "collectors"
            ;;
        "ui")
            build_swift_component "ui"
            ;;
        *)
            error "Unknown component: $component"
            ;;
    esac
}

# Build all components
build_all() {
    log "Building components: ${COMPONENTS[*]}"
    
    if [ "$PARALLEL" = true ]; then
        log "Building in parallel..."
        
        local pids=()
        for component in "${COMPONENTS[@]}"; do
            (
                build_component "$component"
            ) &
            pids+=($!)
        done
        
        # Wait for all builds to complete
        local failed=false
        for pid in "${pids[@]}"; do
            if ! wait "$pid"; then
                failed=true
            fi
        done
        
        if [ "$failed" = true ]; then
            error "One or more builds failed"
        fi
    else
        log "Building serially..."
        
        for component in "${COMPONENTS[@]}"; do
            build_component "$component"
        done
    fi
}

# Verify builds
verify_builds() {
    log "Verifying builds..."
    
    cd "$ROOT_DIR"
    
    # Check ring buffer
    if [[ " ${COMPONENTS[*]} " =~ " ring-buffer " ]]; then
        if [ ! -f "ring-buffer/libringbuffer.a" ]; then
            error "Ring buffer library not found"
        fi
    fi
    
    # Check Rust binaries
    local target_dir="target"
    if [ "$BUILD_TYPE" = "release" ]; then
        target_dir="$target_dir/release"
    else
        target_dir="$target_dir/debug"
    fi
    
    if [[ " ${COMPONENTS[*]} " =~ " packer " ]]; then
        if [ ! -f "packer/$target_dir/chronicle-packer" ]; then
            error "Packer binary not found"
        fi
    fi
    
    if [[ " ${COMPONENTS[*]} " =~ " cli " ]]; then
        if [ ! -f "cli/$target_dir/chronicle" ]; then
            error "CLI binary not found"
        fi
    fi
    
    log "Build verification complete"
}

# Main build function
main() {
    log "Starting Chronicle development build..."
    
    parse_args "$@"
    
    info "Build configuration:"
    info "  Build type: $BUILD_TYPE"
    info "  Components: ${COMPONENTS[*]}"
    info "  Parallel: $PARALLEL"
    info "  Verbose: $VERBOSE"
    info "  Clean: $CLEAN"
    
    cd "$ROOT_DIR"
    
    clean_build
    build_all
    verify_builds
    
    log "Build complete!"
    
    if [ "$BUILD_TYPE" = "release" ]; then
        info "Release binaries available in target/release directories"
    else
        info "Debug binaries available in target/debug directories"
    fi
}

# Run main function
main "$@"