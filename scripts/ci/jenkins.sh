#!/bin/bash

# Chronicle Jenkins Integration Script
# Provides Jenkins-specific functionality and utilities

set -euo pipefail

# Jenkins specific environment
BUILD_NUMBER=${BUILD_NUMBER:-}
WORKSPACE=${WORKSPACE:-$(pwd)}
JOB_NAME=${JOB_NAME:-}
BUILD_URL=${BUILD_URL:-}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
COMMAND=""
VERBOSE=false

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
Usage: $0 COMMAND [OPTIONS]

Jenkins integration script for Chronicle.

COMMANDS:
    setup               Setup environment for Jenkins
    build               Build Chronicle components
    test                Run tests with Jenkins optimizations
    package             Create release packages
    archive             Archive build artifacts

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output

EOF
}

# Parse arguments
parse_args() {
    if [ $# -eq 0 ]; then
        usage
        exit 1
    fi
    
    COMMAND="$1"
    shift
    
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
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done
}

# Setup Jenkins environment
setup_jenkins_environment() {
    log "Setting up Jenkins environment..."
    
    # Set environment variables
    export CARGO_TERM_COLOR=never  # Jenkins doesn't handle colors well
    export CARGO_INCREMENTAL=0
    export RUST_BACKTRACE=1
    
    # Create directories
    mkdir -p "$WORKSPACE/build"
    mkdir -p "$WORKSPACE/dist"
    mkdir -p "$WORKSPACE/test-results"
    
    log "Jenkins environment setup complete"
}

# Build for Jenkins
build_for_jenkins() {
    log "Building Chronicle for Jenkins..."
    
    cd "$WORKSPACE"
    
    # Build with release flag
    if [ -f "scripts/dev/build.sh" ]; then
        ./scripts/dev/build.sh --release
    else
        cargo build --release --all
    fi
    
    log "Jenkins build complete"
}

# Run tests for Jenkins
run_jenkins_tests() {
    log "Running tests for Jenkins..."
    
    cd "$WORKSPACE"
    
    # Run CI tests with Jenkins-specific settings
    if [ -f "scripts/test/run_ci_tests.sh" ]; then
        ./scripts/test/run_ci_tests.sh --output-dir test-results --format xml
    else
        cargo test --all
    fi
    
    log "Jenkins tests complete"
}

# Create packages for Jenkins
create_jenkins_packages() {
    log "Creating packages for Jenkins..."
    
    cd "$WORKSPACE"
    
    local version="build-${BUILD_NUMBER:-$(date +%Y%m%d)}"
    
    # Create packages
    if [ -f "scripts/package/create_zip.sh" ]; then
        ./scripts/package/create_zip.sh --version "$version" --type full
    fi
    
    log "Jenkins packages created"
}

# Archive artifacts for Jenkins
archive_jenkins_artifacts() {
    log "Archiving artifacts for Jenkins..."
    
    cd "$WORKSPACE"
    
    # Create archive directory structure for Jenkins
    mkdir -p artifacts/build
    mkdir -p artifacts/test-results
    mkdir -p artifacts/packages
    
    # Copy build artifacts
    if [ -d "build" ]; then
        cp -r build/* artifacts/build/
    fi
    
    # Copy test results
    if [ -d "test-results" ]; then
        cp -r test-results/* artifacts/test-results/
    fi
    
    # Copy packages
    if [ -d "dist" ]; then
        cp -r dist/* artifacts/packages/
    fi
    
    log "Jenkins artifacts archived"
}

# Execute command
case $COMMAND in
    "setup")
        setup_jenkins_environment
        ;;
    "build")
        build_for_jenkins
        ;;
    "test")
        run_jenkins_tests
        ;;
    "package")
        create_jenkins_packages
        ;;
    "archive")
        archive_jenkins_artifacts
        ;;
    *)
        error "Unknown command: $COMMAND"
        ;;
esac

# Main function
main() {
    parse_args "$@"
    log "Jenkins integration completed"
}

main "$@"