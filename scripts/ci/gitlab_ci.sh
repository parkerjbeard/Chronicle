#!/bin/bash

# Chronicle GitLab CI Integration Script
# Provides GitLab CI-specific functionality and utilities

set -euo pipefail

# GitLab CI specific environment
CI=${CI:-false}
GITLAB_CI=${GITLAB_CI:-false}
CI_PROJECT_DIR=${CI_PROJECT_DIR:-$(pwd)}
CI_COMMIT_TAG=${CI_COMMIT_TAG:-}
CI_COMMIT_REF_NAME=${CI_COMMIT_REF_NAME:-}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
COMMAND=""
VERBOSE=false
CACHE_ENABLED=true
ARTIFACT_UPLOAD=true

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

GitLab CI integration script for Chronicle.

COMMANDS:
    setup               Setup environment for CI
    build               Build Chronicle components
    test                Run tests with CI optimizations
    package             Create release packages
    deploy              Deploy to staging/production

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    --stage STAGE       CI stage (build, test, package, deploy)

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
            --stage)
                CI_STAGE="$2"
                shift 2
                ;;
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done
}

# Setup CI environment
setup_ci_environment() {
    log "Setting up GitLab CI environment..."
    
    # Set environment variables
    export CARGO_TERM_COLOR=always
    export CARGO_INCREMENTAL=0
    export RUST_BACKTRACE=1
    
    # Install dependencies
    if command -v apt-get &> /dev/null; then
        apt-get update -qq
        apt-get install -y build-essential curl
    fi
    
    # Create directories
    mkdir -p "$CI_PROJECT_DIR/build"
    mkdir -p "$CI_PROJECT_DIR/dist"
    mkdir -p "$CI_PROJECT_DIR/test-results"
    
    log "GitLab CI environment setup complete"
}

# Build for GitLab CI
build_for_ci() {
    log "Building Chronicle for GitLab CI..."
    
    cd "$CI_PROJECT_DIR"
    
    # Build with release flag
    if [ -f "scripts/dev/build.sh" ]; then
        ./scripts/dev/build.sh --release
    else
        cargo build --release --all
    fi
    
    log "GitLab CI build complete"
}

# Run tests for GitLab CI
run_ci_tests() {
    log "Running tests for GitLab CI..."
    
    cd "$CI_PROJECT_DIR"
    
    # Run CI tests
    if [ -f "scripts/test/run_ci_tests.sh" ]; then
        ./scripts/test/run_ci_tests.sh --output-dir test-results
    else
        cargo test --all
    fi
    
    log "GitLab CI tests complete"
}

# Create packages for GitLab CI
create_packages() {
    log "Creating packages for GitLab CI..."
    
    cd "$CI_PROJECT_DIR"
    
    local version="${CI_COMMIT_TAG:-dev-$(date +%Y%m%d)}"
    
    # Create ZIP package
    if [ -f "scripts/package/create_zip.sh" ]; then
        ./scripts/package/create_zip.sh --version "$version" --type full
    fi
    
    log "GitLab CI packages created"
}

# Deploy for GitLab CI
deploy_for_ci() {
    log "Deploying for GitLab CI..."
    
    # Deployment logic would go here
    # This could include uploading to package registries, etc.
    
    log "GitLab CI deployment complete"
}

# Execute command
case $COMMAND in
    "setup")
        setup_ci_environment
        ;;
    "build")
        build_for_ci
        ;;
    "test")
        run_ci_tests
        ;;
    "package")
        create_packages
        ;;
    "deploy")
        deploy_for_ci
        ;;
    *)
        error "Unknown command: $COMMAND"
        ;;
esac

# Main function
main() {
    parse_args "$@"
    log "GitLab CI integration completed"
}

main "$@"