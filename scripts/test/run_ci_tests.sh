#!/bin/bash

# Chronicle CI Tests Runner
# Optimized test suite for Continuous Integration environments

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
COVERAGE=false
FAIL_FAST=true
OUTPUT_DIR="$ROOT_DIR/test-results/ci"
CI_MODE=""
TIMEOUT=900  # 15 minutes for CI
PARALLEL=true

# CI Environment Detection
detect_ci_environment() {
    if [ -n "${GITHUB_ACTIONS:-}" ]; then
        CI_MODE="github"
    elif [ -n "${GITLAB_CI:-}" ]; then
        CI_MODE="gitlab"
    elif [ -n "${JENKINS_URL:-}" ]; then
        CI_MODE="jenkins"
    elif [ -n "${CI:-}" ]; then
        CI_MODE="generic"
    else
        CI_MODE="local"
    fi
}

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

# CI-specific logging
ci_log() {
    case $CI_MODE in
        "github")
            echo "::notice::$1"
            ;;
        "gitlab")
            echo "NOTICE: $1"
            ;;
        *)
            info "$1"
            ;;
    esac
}

ci_error() {
    case $CI_MODE in
        "github")
            echo "::error::$1"
            ;;
        "gitlab")
            echo "ERROR: $1"
            ;;
        *)
            error "$1"
            ;;
    esac
}

ci_warning() {
    case $CI_MODE in
        "github")
            echo "::warning::$1"
            ;;
        "gitlab")
            echo "WARNING: $1"
            ;;
        *)
            warn "$1"
            ;;
    esac
}

# Show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Run Chronicle CI-optimized test suite.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -c, --coverage          Generate coverage reports
    --no-fail-fast          Don't stop on first failure
    --output-dir DIR        Output directory for test results
    --timeout SECONDS       Test timeout in seconds (default: 900)
    --serial                Run tests serially
    --quick                 Run only quick tests
    --smoke                 Run smoke tests only
    --full                  Run full test suite

EXAMPLES:
    $0                      # Run CI test suite
    $0 --coverage           # Run with coverage
    $0 --quick              # Quick tests only
    $0 --smoke              # Smoke tests only

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
            -c|--coverage)
                COVERAGE=true
                shift
                ;;
            --no-fail-fast)
                FAIL_FAST=false
                shift
                ;;
            --output-dir)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            --timeout)
                TIMEOUT="$2"
                shift 2
                ;;
            --serial)
                PARALLEL=false
                shift
                ;;
            --quick)
                TEST_LEVEL="quick"
                shift
                ;;
            --smoke)
                TEST_LEVEL="smoke"
                shift
                ;;
            --full)
                TEST_LEVEL="full"
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
    
    # Default test level based on CI environment
    if [ -z "${TEST_LEVEL:-}" ]; then
        if [ "$CI_MODE" = "local" ]; then
            TEST_LEVEL="full"
        else
            TEST_LEVEL="standard"
        fi
    fi
}

# Setup CI test environment
setup_ci_test_env() {
    log "Setting up CI test environment..."
    
    # Detect CI environment
    detect_ci_environment
    ci_log "Detected CI environment: $CI_MODE"
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    # Set CI-optimized environment variables
    export RUST_BACKTRACE=1
    export RUST_LOG=warn
    export CHRONICLE_CI_TEST=1
    export CHRONICLE_TEST_TIMEOUT="$TIMEOUT"
    
    # CI-specific optimizations
    case $CI_MODE in
        "github")
            export CARGO_TERM_COLOR=always
            export CARGO_INCREMENTAL=0
            export RUSTC_WRAPPER=""
            ;;
        "gitlab")
            export CARGO_TERM_COLOR=always
            export CI_PROJECT_DIR="${CI_PROJECT_DIR:-$ROOT_DIR}"
            ;;
        "jenkins")
            export CARGO_TERM_COLOR=never
            ;;
    esac
    
    # Optimize for CI performance
    if [ "$CI_MODE" != "local" ]; then
        export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-$(nproc)}
        export MAKEFLAGS="-j$(nproc)"
    fi
    
    log "CI test environment ready"
}

# Check CI prerequisites
check_ci_prerequisites() {
    log "Checking CI prerequisites..."
    
    # Check for required tools
    local required_tools=("cargo" "rustc")
    
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            ci_error "Required tool not found: $tool"
        fi
    done
    
    # Check Rust version
    local rust_version=$(rustc --version)
    ci_log "Rust version: $rust_version"
    
    # Check for Xcode (on macOS)
    if [[ "$OSTYPE" = "darwin"* ]]; then
        if ! xcodebuild -version &> /dev/null; then
            ci_warning "Xcode not available - Swift tests will be skipped"
        else
            local xcode_version=$(xcodebuild -version | head -1)
            ci_log "Xcode version: $xcode_version"
        fi
    fi
    
    # Check available disk space
    local available_space=$(df . | tail -1 | awk '{print $4}')
    if [ "$available_space" -lt 1000000 ]; then  # Less than 1GB
        ci_warning "Low disk space available: ${available_space}KB"
    fi
    
    log "Prerequisites check completed"
}

# Run smoke tests
run_ci_smoke_tests() {
    log "Running CI smoke tests..."
    
    local smoke_dir="$OUTPUT_DIR/smoke"
    mkdir -p "$smoke_dir"
    
    local smoke_failed=false
    
    # Test ring buffer compilation
    if [ -d "$ROOT_DIR/ring-buffer" ]; then
        cd "$ROOT_DIR/ring-buffer"
        if make clean && make > "$smoke_dir/ring_buffer_build.log" 2>&1; then
            ci_log "Ring buffer build: PASSED"
        else
            ci_warning "Ring buffer build: FAILED"
            smoke_failed=true
        fi
    fi
    
    # Test Rust compilation
    local rust_projects=("packer" "cli" "benchmarks")
    for project in "${rust_projects[@]}"; do
        if [ -d "$ROOT_DIR/$project" ]; then
            cd "$ROOT_DIR/$project"
            if cargo check > "$smoke_dir/${project}_check.log" 2>&1; then
                ci_log "$project check: PASSED"
            else
                ci_warning "$project check: FAILED"
                smoke_failed=true
            fi
        fi
    done
    
    # Test Swift compilation (if available)
    if [[ "$OSTYPE" = "darwin"* ]] && command -v xcodebuild &> /dev/null; then
        cd "$ROOT_DIR"
        if xcodebuild -workspace Chronicle.xcworkspace -scheme ChronicleCollectors -configuration Debug -quiet build > "$smoke_dir/swift_build.log" 2>&1; then
            ci_log "Swift build: PASSED"
        else
            ci_warning "Swift build: FAILED"
            smoke_failed=true
        fi
    fi
    
    if [ "$smoke_failed" = true ]; then
        if [ "$FAIL_FAST" = true ]; then
            ci_error "Smoke tests failed"
        else
            return 1
        fi
    fi
    
    log "CI smoke tests completed"
    return 0
}

# Run quick tests
run_ci_quick_tests() {
    log "Running CI quick tests..."
    
    local quick_dir="$OUTPUT_DIR/quick"
    mkdir -p "$quick_dir"
    
    # Run unit tests for core components
    local unit_script="$SCRIPT_DIR/run_unit_tests.sh"
    local unit_flags="--output-dir $quick_dir/unit"
    
    if [ "$VERBOSE" = true ]; then
        unit_flags="$unit_flags --verbose"
    fi
    
    if [ "$PARALLEL" = true ]; then
        unit_flags="$unit_flags --parallel"
    else
        unit_flags="$unit_flags --serial"
    fi
    
    # Only test core components for quick tests
    unit_flags="$unit_flags --ring-buffer --packer --cli"
    
    if ! timeout "$TIMEOUT" "$unit_script" $unit_flags > "$quick_dir/unit_tests.log" 2>&1; then
        ci_warning "Quick unit tests failed"
        if [ "$FAIL_FAST" = true ]; then
            ci_error "Quick tests failed"
        fi
        return 1
    fi
    
    log "CI quick tests completed"
    return 0
}

# Run standard CI tests
run_ci_standard_tests() {
    log "Running CI standard tests..."
    
    local standard_dir="$OUTPUT_DIR/standard"
    mkdir -p "$standard_dir"
    
    local test_results=()
    
    # Run unit tests
    local unit_script="$SCRIPT_DIR/run_unit_tests.sh"
    local unit_flags="--output-dir $standard_dir/unit"
    
    if [ "$VERBOSE" = true ]; then
        unit_flags="$unit_flags --verbose"
    fi
    
    if [ "$COVERAGE" = true ]; then
        unit_flags="$unit_flags --coverage"
    fi
    
    if [ "$PARALLEL" = true ]; then
        unit_flags="$unit_flags --parallel"
    else
        unit_flags="$unit_flags --serial"
    fi
    
    if timeout "$TIMEOUT" "$unit_script" $unit_flags > "$standard_dir/unit_tests.log" 2>&1; then
        test_results+=("unit:PASSED")
        ci_log "Unit tests: PASSED"
    else
        test_results+=("unit:FAILED")
        ci_warning "Unit tests: FAILED"
    fi
    
    # Run integration tests
    local integration_script="$SCRIPT_DIR/run_integration_tests.sh"
    local integration_flags="--output-dir $standard_dir/integration"
    integration_flags="$integration_flags --timeout $((TIMEOUT / 2))"
    
    if [ "$VERBOSE" = true ]; then
        integration_flags="$integration_flags --verbose"
    fi
    
    if [ "$PARALLEL" = true ]; then
        integration_flags="$integration_flags --parallel"
    else
        integration_flags="$integration_flags --serial"
    fi
    
    # Only run essential integration tests
    integration_flags="$integration_flags --ring-buffer --packer-cli --api"
    
    if timeout "$TIMEOUT" "$integration_script" $integration_flags > "$standard_dir/integration_tests.log" 2>&1; then
        test_results+=("integration:PASSED")
        ci_log "Integration tests: PASSED"
    else
        test_results+=("integration:FAILED")
        ci_warning "Integration tests: FAILED"
    fi
    
    # Check results
    local failed_tests=()
    for result in "${test_results[@]}"; do
        local test_type="${result%:*}"
        local test_status="${result#*:}"
        
        if [ "$test_status" = "FAILED" ]; then
            failed_tests+=("$test_type")
        fi
    done
    
    if [ ${#failed_tests[@]} -gt 0 ]; then
        if [ "$FAIL_FAST" = true ]; then
            ci_error "Standard tests failed: ${failed_tests[*]}"
        else
            return 1
        fi
    fi
    
    log "CI standard tests completed"
    return 0
}

# Run full CI tests
run_ci_full_tests() {
    log "Running CI full tests..."
    
    local full_dir="$OUTPUT_DIR/full"
    mkdir -p "$full_dir"
    
    # Run all test suites
    local all_tests_script="$SCRIPT_DIR/run_all_tests.sh"
    local all_tests_flags="--output-dir $full_dir"
    
    if [ "$VERBOSE" = true ]; then
        all_tests_flags="$all_tests_flags --verbose"
    fi
    
    if [ "$COVERAGE" = true ]; then
        all_tests_flags="$all_tests_flags --coverage"
    fi
    
    if [ "$PARALLEL" = true ]; then
        all_tests_flags="$all_tests_flags --parallel"
    else
        all_tests_flags="$all_tests_flags --serial"
    fi
    
    if [ "$FAIL_FAST" = true ]; then
        all_tests_flags="$all_tests_flags --fail-fast"
    fi
    
    if ! timeout "$TIMEOUT" "$all_tests_script" $all_tests_flags > "$full_dir/all_tests.log" 2>&1; then
        ci_error "Full test suite failed"
    fi
    
    log "CI full tests completed"
    return 0
}

# Run tests based on level
run_ci_tests() {
    case $TEST_LEVEL in
        "smoke")
            run_ci_smoke_tests
            ;;
        "quick")
            run_ci_smoke_tests
            run_ci_quick_tests
            ;;
        "standard")
            run_ci_smoke_tests
            run_ci_standard_tests
            ;;
        "full")
            run_ci_smoke_tests
            run_ci_full_tests
            ;;
        *)
            error "Unknown test level: $TEST_LEVEL"
            ;;
    esac
}

# Generate CI test report
generate_ci_report() {
    log "Generating CI test report..."
    
    local report_file="$OUTPUT_DIR/ci-report.txt"
    
    cat > "$report_file" << EOF
Chronicle CI Test Report
Generated: $(date)

CI Environment: $CI_MODE
Test Level: $TEST_LEVEL
Configuration:
  Parallel: $PARALLEL
  Coverage: $COVERAGE
  Fail Fast: $FAIL_FAST
  Timeout: ${TIMEOUT}s

Test Results:
EOF
    
    # Add test results based on level
    case $TEST_LEVEL in
        "smoke")
            if [ -d "$OUTPUT_DIR/smoke" ]; then
                echo "  Smoke Tests: Completed" >> "$report_file"
            fi
            ;;
        "quick")
            if [ -d "$OUTPUT_DIR/smoke" ]; then
                echo "  Smoke Tests: Completed" >> "$report_file"
            fi
            if [ -d "$OUTPUT_DIR/quick" ]; then
                echo "  Quick Tests: Completed" >> "$report_file"
            fi
            ;;
        "standard")
            if [ -d "$OUTPUT_DIR/smoke" ]; then
                echo "  Smoke Tests: Completed" >> "$report_file"
            fi
            if [ -d "$OUTPUT_DIR/standard" ]; then
                echo "  Standard Tests: Completed" >> "$report_file"
            fi
            ;;
        "full")
            if [ -d "$OUTPUT_DIR/smoke" ]; then
                echo "  Smoke Tests: Completed" >> "$report_file"
            fi
            if [ -d "$OUTPUT_DIR/full" ]; then
                echo "  Full Test Suite: Completed" >> "$report_file"
            fi
            ;;
    esac
    
    # Add coverage information if available
    if [ "$COVERAGE" = true ]; then
        echo "" >> "$report_file"
        echo "Coverage Reports:" >> "$report_file"
        find "$OUTPUT_DIR" -name "coverage" -type d | while read -r coverage_dir; do
            echo "  $(basename "$(dirname "$coverage_dir")"): $coverage_dir" >> "$report_file"
        done
    fi
    
    # Add CI-specific outputs
    case $CI_MODE in
        "github")
            echo "::set-output name=report_path::$report_file"
            ;;
        "gitlab")
            echo "CI_REPORT_PATH=$report_file" >> "$OUTPUT_DIR/ci_variables"
            ;;
    esac
    
    log "CI test report saved to $report_file"
}

# Upload test artifacts (CI-specific)
upload_test_artifacts() {
    log "Processing test artifacts..."
    
    case $CI_MODE in
        "github")
            # GitHub Actions artifacts are handled by workflow
            echo "::set-output name=artifacts_path::$OUTPUT_DIR"
            ;;
        "gitlab")
            # GitLab CI artifacts are defined in .gitlab-ci.yml
            echo "Test artifacts available in $OUTPUT_DIR"
            ;;
        "jenkins")
            # Jenkins artifacts are handled by pipeline
            echo "Test artifacts available in $OUTPUT_DIR"
            ;;
        *)
            echo "Test artifacts available in $OUTPUT_DIR"
            ;;
    esac
}

# Main CI test function
main() {
    log "Starting Chronicle CI tests..."
    
    parse_args "$@"
    
    ci_log "CI test configuration:"
    ci_log "  CI Environment: $CI_MODE"
    ci_log "  Test Level: $TEST_LEVEL"
    ci_log "  Parallel: $PARALLEL"
    ci_log "  Coverage: $COVERAGE"
    ci_log "  Fail Fast: $FAIL_FAST"
    ci_log "  Timeout: ${TIMEOUT}s"
    ci_log "  Output Directory: $OUTPUT_DIR"
    
    setup_ci_test_env
    check_ci_prerequisites
    run_ci_tests
    generate_ci_report
    upload_test_artifacts
    
    log "CI tests completed successfully!"
    ci_log "Test results available in: $OUTPUT_DIR"
}

# Run main function
main "$@"