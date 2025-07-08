#!/bin/bash

# Chronicle GitHub Actions Integration Script
# Provides GitHub Actions-specific functionality and utilities

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# GitHub Actions specific environment
CI=${CI:-false}
GITHUB_ACTIONS=${GITHUB_ACTIONS:-false}
GITHUB_WORKSPACE=${GITHUB_WORKSPACE:-$(pwd)}
GITHUB_OUTPUT=${GITHUB_OUTPUT:-/dev/stdout}
GITHUB_STEP_SUMMARY=${GITHUB_STEP_SUMMARY:-/dev/stdout}

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
ROOT_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Configuration
COMMAND=""
VERBOSE=false
CACHE_ENABLED=true
ARTIFACT_UPLOAD=true

# Logging with GitHub Actions annotations
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
    if [ "$GITHUB_ACTIONS" = "true" ]; then
        echo "::notice::$1"
    fi
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
    if [ "$GITHUB_ACTIONS" = "true" ]; then
        echo "::warning::$1"
    fi
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    if [ "$GITHUB_ACTIONS" = "true" ]; then
        echo "::error::$1"
    fi
    exit 1
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}"
}

# Set GitHub Actions output
set_output() {
    local name="$1"
    local value="$2"
    
    if [ "$GITHUB_ACTIONS" = "true" ]; then
        echo "$name=$value" >> "$GITHUB_OUTPUT"
    fi
    
    if [ "$VERBOSE" = true ]; then
        info "Output: $name=$value"
    fi
}

# Add to step summary
add_summary() {
    local content="$1"
    
    if [ "$GITHUB_ACTIONS" = "true" ]; then
        echo "$content" >> "$GITHUB_STEP_SUMMARY"
    fi
}

# Show usage
usage() {
    cat << EOF
Usage: $0 COMMAND [OPTIONS]

GitHub Actions integration script for Chronicle.

COMMANDS:
    setup               Setup environment for CI
    build               Build Chronicle components
    test                Run tests with CI optimizations
    package             Create release packages
    upload-artifacts    Upload build artifacts
    create-release      Create GitHub release
    cache-save          Save cache
    cache-restore       Restore cache

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    --no-cache          Disable caching
    --no-artifacts      Disable artifact upload

EXAMPLES:
    $0 setup
    $0 build --verbose
    $0 test --no-cache
    $0 package

EOF
}

# Parse command line arguments
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
            --no-cache)
                CACHE_ENABLED=false
                shift
                ;;
            --no-artifacts)
                ARTIFACT_UPLOAD=false
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
}

# Setup CI environment
setup_ci_environment() {
    log "Setting up GitHub Actions environment..."
    
    # Set GitHub Actions specific environment variables
    export CARGO_TERM_COLOR=always
    export CARGO_INCREMENTAL=0
    export RUSTC_WRAPPER=""
    export RUST_BACKTRACE=1
    
    # Optimize for CI
    export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-$(nproc)}
    export MAKEFLAGS="-j$(nproc)"
    
    # Create necessary directories
    mkdir -p "$GITHUB_WORKSPACE/build"
    mkdir -p "$GITHUB_WORKSPACE/dist"
    mkdir -p "$GITHUB_WORKSPACE/test-results"
    
    # Install additional tools if needed
    if ! command -v create-dmg &> /dev/null; then
        log "Installing create-dmg..."
        brew install create-dmg
    fi
    
    # Set outputs
    set_output "rust-version" "$(rustc --version)"
    set_output "setup-complete" "true"
    
    # Add to summary
    add_summary "## Setup Complete"
    add_summary "- Environment configured for CI"
    add_summary "- Build directories created"
    add_summary "- Tools installed"
    
    log "GitHub Actions environment setup complete"
}

# Build Chronicle for CI
build_for_ci() {
    log "Building Chronicle for CI..."
    
    cd "$GITHUB_WORKSPACE"
    
    # Use the main build script
    local build_script="$ROOT_DIR/scripts/dev/build.sh"
    local build_flags="--release"
    
    if [ "$VERBOSE" = true ]; then
        build_flags="$build_flags --verbose"
    fi
    
    if ! "$build_script" $build_flags; then
        error "Build failed"
    fi
    
    # Collect build artifacts info
    local artifacts=()
    
    # Find built binaries
    while IFS= read -r -d '' file; do
        artifacts+=("$(basename "$file")")
    done < <(find . -name "chronicle" -o -name "chronicle-packer" -print0)
    
    # Find built apps
    while IFS= read -r -d '' app; do
        artifacts+=("$(basename "$app")")
    done < <(find . -name "*.app" -type d -print0)
    
    # Set outputs
    set_output "build-status" "success"
    set_output "artifacts" "$(IFS=,; echo "${artifacts[*]}")"
    
    # Add to summary
    add_summary "## Build Results"
    add_summary "✅ Build completed successfully"
    add_summary ""
    add_summary "### Artifacts Built:"
    for artifact in "${artifacts[@]}"; do
        add_summary "- $artifact"
    done
    
    log "CI build complete"
}

# Run tests for CI
run_ci_tests() {
    log "Running tests for CI..."
    
    cd "$GITHUB_WORKSPACE"
    
    # Use the CI test script
    local test_script="$ROOT_DIR/scripts/test/run_ci_tests.sh"
    local test_flags="--output-dir test-results"
    
    if [ "$VERBOSE" = true ]; then
        test_flags="$test_flags --verbose"
    fi
    
    # Run tests and capture results
    local test_status="success"
    if ! "$test_script" $test_flags; then
        test_status="failure"
    fi
    
    # Parse test results
    local test_summary=""
    if [ -f "test-results/ci-report.txt" ]; then
        test_summary=$(cat "test-results/ci-report.txt")
    fi
    
    # Set outputs
    set_output "test-status" "$test_status"
    set_output "test-results-path" "test-results"
    
    # Add to summary
    add_summary "## Test Results"
    if [ "$test_status" = "success" ]; then
        add_summary "✅ All tests passed"
    else
        add_summary "❌ Some tests failed"
    fi
    
    if [ -n "$test_summary" ]; then
        add_summary ""
        add_summary "### Test Summary"
        add_summary "\`\`\`"
        add_summary "$test_summary"
        add_summary "\`\`\`"
    fi
    
    log "CI tests complete"
    
    if [ "$test_status" = "failure" ]; then
        exit 1
    fi
}

# Create release packages
create_release_packages() {
    log "Creating release packages for CI..."
    
    cd "$GITHUB_WORKSPACE"
    
    # Get version from git tag or environment
    local version="${GITHUB_REF_NAME#v}"
    if [ -z "$version" ] || [ "$version" = "$GITHUB_REF_NAME" ]; then
        version="$(date +%Y%m%d)-dev"
    fi
    
    # Create DMG
    local dmg_script="$ROOT_DIR/scripts/package/create_dmg.sh"
    if [ -f "$dmg_script" ]; then
        log "Creating DMG package..."
        if ! "$dmg_script" --version "$version" --output-dir dist/dmg; then
            warn "DMG creation failed"
        fi
    fi
    
    # Create PKG
    local pkg_script="$ROOT_DIR/scripts/package/create_pkg.sh"
    if [ -f "$pkg_script" ]; then
        log "Creating PKG package..."
        if ! "$pkg_script" --version "$version" --output-dir dist/pkg; then
            warn "PKG creation failed"
        fi
    fi
    
    # Create ZIP
    local zip_script="$ROOT_DIR/scripts/package/create_zip.sh"
    if [ -f "$zip_script" ]; then
        log "Creating ZIP package..."
        if ! "$zip_script" --version "$version" --type full --output-dir dist/zip; then
            warn "ZIP creation failed"
        fi
    fi
    
    # List created packages
    local packages=()
    while IFS= read -r -d '' file; do
        packages+=("$(basename "$file")")
    done < <(find dist -name "*.dmg" -o -name "*.pkg" -o -name "*.zip" -print0)
    
    # Set outputs
    set_output "package-version" "$version"
    set_output "packages" "$(IFS=,; echo "${packages[*]}")"
    set_output "packages-path" "dist"
    
    # Add to summary
    add_summary "## Release Packages"
    add_summary "Version: \`$version\`"
    add_summary ""
    add_summary "### Packages Created:"
    for package in "${packages[@]}"; do
        add_summary "- $package"
    done
    
    log "Release packages created"
}

# Upload artifacts to GitHub Actions
upload_artifacts() {
    if [ "$ARTIFACT_UPLOAD" = false ]; then
        log "Artifact upload disabled"
        return 0
    fi
    
    log "Uploading artifacts..."
    
    # This function sets up artifacts for upload
    # The actual upload is done by the upload-artifact action
    
    local artifact_paths=()
    
    # Collect build artifacts
    if [ -d "build" ]; then
        artifact_paths+=("build")
    fi
    
    # Collect test results
    if [ -d "test-results" ]; then
        artifact_paths+=("test-results")
    fi
    
    # Collect packages
    if [ -d "dist" ]; then
        artifact_paths+=("dist")
    fi
    
    # Set outputs for action consumption
    set_output "artifact-paths" "$(IFS=,; echo "${artifact_paths[*]}")"
    set_output "upload-artifacts" "true"
    
    log "Artifact upload prepared"
}

# Create GitHub release
create_github_release() {
    log "Creating GitHub release..."
    
    # Check if this is a tag build
    if [[ ! "$GITHUB_REF" =~ refs/tags/ ]]; then
        warn "Not a tag build, skipping release creation"
        return 0
    fi
    
    local tag_name="${GITHUB_REF#refs/tags/}"
    local version="${tag_name#v}"
    
    # Check if release already exists
    if gh release view "$tag_name" &> /dev/null; then
        warn "Release $tag_name already exists"
        return 0
    fi
    
    # Generate release notes
    local release_notes="Release $version"
    if [ -f "CHANGELOG.md" ]; then
        # Extract changelog for this version
        release_notes=$(awk "/## \\[$version\\]/,/## \\[/{if(/## \\[/ && !/## \\[$version\\]/) exit; if(!/## \\[$version\\]/) print}" CHANGELOG.md || echo "Release $version")
    fi
    
    # Create release
    local release_flags=""
    release_flags="--title \"Chronicle $version\""
    release_flags="$release_flags --notes \"$release_notes\""
    
    # Add packages as assets
    local packages=()
    while IFS= read -r -d '' file; do
        packages+=("$file")
    done < <(find dist -name "*.dmg" -o -name "*.pkg" -o -name "*.zip" -print0)
    
    if [ ${#packages[@]} -gt 0 ]; then
        for package in "${packages[@]}"; do
            release_flags="$release_flags \"$package\""
        done
    fi
    
    # Create the release
    eval "gh release create \"$tag_name\" $release_flags"
    
    # Set outputs
    set_output "release-tag" "$tag_name"
    set_output "release-url" "https://github.com/$GITHUB_REPOSITORY/releases/tag/$tag_name"
    
    # Add to summary
    add_summary "## GitHub Release Created"
    add_summary "- Tag: \`$tag_name\`"
    add_summary "- Version: \`$version\`"
    add_summary "- [View Release](https://github.com/$GITHUB_REPOSITORY/releases/tag/$tag_name)"
    
    log "GitHub release created: $tag_name"
}

# Save cache
save_cache() {
    if [ "$CACHE_ENABLED" = false ]; then
        log "Cache disabled"
        return 0
    fi
    
    log "Preparing cache save..."
    
    # Set up cache paths for GitHub Actions cache action
    local cache_paths=()
    
    # Rust cache
    if [ -d "$HOME/.cargo" ]; then
        cache_paths+=("~/.cargo/registry")
        cache_paths+=("~/.cargo/git")
    fi
    
    # Target directories
    while IFS= read -r -d '' dir; do
        cache_paths+=("$(realpath --relative-to="$GITHUB_WORKSPACE" "$dir")")
    done < <(find . -name "target" -type d -print0)
    
    # Xcode derived data
    if [ -d "$HOME/Library/Developer/Xcode/DerivedData" ]; then
        cache_paths+=("~/Library/Developer/Xcode/DerivedData")
    fi
    
    # Set outputs for cache action
    set_output "cache-paths" "$(IFS=$'\n'; echo "${cache_paths[*]}")"
    set_output "cache-enabled" "true"
    
    log "Cache save prepared"
}

# Restore cache
restore_cache() {
    if [ "$CACHE_ENABLED" = false ]; then
        log "Cache disabled"
        return 0
    fi
    
    log "Cache restore will be handled by GitHub Actions cache action"
    
    # Set output to indicate cache should be restored
    set_output "cache-restore" "true"
}

# Execute command
execute_command() {
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
            create_release_packages
            ;;
        "upload-artifacts")
            upload_artifacts
            ;;
        "create-release")
            create_github_release
            ;;
        "cache-save")
            save_cache
            ;;
        "cache-restore")
            restore_cache
            ;;
        *)
            error "Unknown command: $COMMAND"
            ;;
    esac
}

# Main function
main() {
    log "Starting GitHub Actions integration script..."
    
    # Verify we're in GitHub Actions if expected
    if [ "${CI:-}" = "true" ] && [ "${GITHUB_ACTIONS:-}" != "true" ]; then
        warn "Running in CI but not GitHub Actions"
    fi
    
    parse_args "$@"
    
    info "Command: $COMMAND"
    info "Verbose: $VERBOSE"
    info "Cache Enabled: $CACHE_ENABLED"
    info "Artifact Upload: $ARTIFACT_UPLOAD"
    
    execute_command
    
    log "GitHub Actions integration script completed"
}

# Run main function
main "$@"