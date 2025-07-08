#!/bin/bash
set -euo pipefail

# Chronicle CI Test Suite
# Optimized testing script for Continuous Integration environments

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CI_RESULTS_DIR="$SCRIPT_DIR/ci_results"
LOG_FILE="$CI_RESULTS_DIR/ci_test_$(date +%Y%m%d_%H%M%S).log"

# CI-specific settings
CI_TIMEOUT=1800  # 30 minutes total timeout
TEST_TIMEOUT=300 # 5 minutes per test suite
PARALLEL_JOBS=${CI_PARALLEL_JOBS:-2}
COVERAGE_THRESHOLD=${COVERAGE_THRESHOLD:-80}

# Test configuration for CI
RUN_UNIT_TESTS=true
RUN_INTEGRATION_TESTS=true
RUN_PERFORMANCE_TESTS=true
RUN_STRESS_TESTS=false  # Disabled in CI by default
RUN_SWIFT_TESTS=true
GENERATE_COVERAGE=true
QUICK_MODE=false

# Parse CI environment variables
parse_ci_config() {
    # GitHub Actions
    if [[ -n "${GITHUB_ACTIONS:-}" ]]; then
        log_info "Detected GitHub Actions environment"
        CI_ENVIRONMENT="github"
        CI_BRANCH="${GITHUB_REF_NAME:-unknown}"
        CI_COMMIT="${GITHUB_SHA:-unknown}"
        
        # Use GitHub-specific configurations
        if [[ "${GITHUB_EVENT_NAME:-}" == "pull_request" ]]; then
            QUICK_MODE=true
            RUN_PERFORMANCE_TESTS=false
        fi
    
    # GitLab CI
    elif [[ -n "${GITLAB_CI:-}" ]]; then
        log_info "Detected GitLab CI environment"
        CI_ENVIRONMENT="gitlab"
        CI_BRANCH="${CI_COMMIT_REF_NAME:-unknown}"
        CI_COMMIT="${CI_COMMIT_SHA:-unknown}"
    
    # Jenkins
    elif [[ -n "${JENKINS_URL:-}" ]]; then
        log_info "Detected Jenkins environment"
        CI_ENVIRONMENT="jenkins"
        CI_BRANCH="${GIT_BRANCH:-unknown}"
        CI_COMMIT="${GIT_COMMIT:-unknown}"
    
    # Generic CI
    elif [[ -n "${CI:-}" ]]; then
        log_info "Detected generic CI environment"
        CI_ENVIRONMENT="generic"
        CI_BRANCH="${CI_BRANCH:-unknown}"
        CI_COMMIT="${CI_COMMIT:-unknown}"
    
    else
        log_info "Running in local development environment"
        CI_ENVIRONMENT="local"
        CI_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
        CI_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
    fi
    
    log_info "CI Environment: $CI_ENVIRONMENT"
    log_info "Branch: $CI_BRANCH"
    log_info "Commit: ${CI_COMMIT:0:8}"
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --quick)
                QUICK_MODE=true
                RUN_PERFORMANCE_TESTS=false
                RUN_STRESS_TESTS=false
                shift
                ;;
            --coverage-threshold)
                COVERAGE_THRESHOLD="$2"
                shift 2
                ;;
            --parallel-jobs)
                PARALLEL_JOBS="$2"
                shift 2
                ;;
            --timeout)
                CI_TIMEOUT="$2"
                shift 2
                ;;
            --enable-stress)
                RUN_STRESS_TESTS=true
                shift
                ;;
            --help)
                show_ci_help
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                show_ci_help
                exit 1
                ;;
        esac
    done
}

show_ci_help() {
    cat << EOF
Chronicle CI Test Suite

Usage: $0 [OPTIONS]

Options:
    --quick                 Enable quick mode (skip performance tests)
    --coverage-threshold N  Set coverage threshold (default: 80)
    --parallel-jobs N       Set number of parallel jobs (default: 2)
    --timeout N             Set total timeout in seconds (default: 1800)
    --enable-stress         Enable stress tests (disabled by default)
    --help                  Show this help message

Environment Variables:
    CI_PARALLEL_JOBS       Number of parallel jobs
    COVERAGE_THRESHOLD     Coverage threshold percentage
    RUST_LOG              Rust logging level
    GITHUB_ACTIONS        Set when running on GitHub Actions
    GITLAB_CI             Set when running on GitLab CI
    JENKINS_URL           Set when running on Jenkins

Examples:
    $0                     # Run standard CI tests
    $0 --quick             # Run quick tests for PR validation
    $0 --enable-stress     # Run all tests including stress tests
EOF
}

# Logging functions
log() {
    echo -e "${GREEN}[CI $(date +'%H:%M:%S')] $1${NC}" | tee -a "$LOG_FILE"
}

log_warn() {
    echo -e "${YELLOW}[CI $(date +'%H:%M:%S')] WARNING: $1${NC}" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[CI $(date +'%H:%M:%S')] ERROR: $1${NC}" | tee -a "$LOG_FILE"
}

log_info() {
    echo -e "${BLUE}[CI $(date +'%H:%M:%S')] INFO: $1${NC}" | tee -a "$LOG_FILE"
}

# Setup CI environment
setup_ci_environment() {
    log "Setting up CI test environment..."
    
    # Create results directory
    mkdir -p "$CI_RESULTS_DIR"
    
    # Initialize log file
    {
        echo "Chronicle CI Test Suite - $(date)"
        echo "======================================"
        echo "Environment: $CI_ENVIRONMENT"
        echo "Branch: $CI_BRANCH"
        echo "Commit: $CI_COMMIT"
        echo "Quick Mode: $QUICK_MODE"
        echo "Coverage Threshold: $COVERAGE_THRESHOLD%"
        echo "Parallel Jobs: $PARALLEL_JOBS"
        echo ""
    } > "$LOG_FILE"
    
    # Set CI-optimized environment variables
    export RUST_LOG=${RUST_LOG:-warn}
    export RUST_BACKTRACE=1
    export CHRONICLE_TEST_MODE=1
    export CHRONICLE_CI_MODE=1
    export CHRONICLE_TEST_DATA_DIR="$CI_RESULTS_DIR/test_data"
    export CARGO_TERM_COLOR=always
    
    # Create test data directory
    mkdir -p "$CHRONICLE_TEST_DATA_DIR"
    
    # Set resource limits for CI
    setup_resource_limits
    
    # Install CI-specific dependencies
    install_ci_dependencies
}

setup_resource_limits() {
    log_info "Setting up resource limits for CI..."
    
    # Set memory limits (if supported)
    if command -v ulimit &> /dev/null; then
        # Limit virtual memory to 2GB
        ulimit -v 2097152 2>/dev/null || log_warn "Could not set memory limit"
        
        # Limit CPU time
        ulimit -t $((CI_TIMEOUT + 300)) 2>/dev/null || log_warn "Could not set CPU time limit"
    fi
    
    # Set up timeout wrapper for individual commands
    if command -v timeout &> /dev/null; then
        TIMEOUT_CMD="timeout $TEST_TIMEOUT"
    elif command -v gtimeout &> /dev/null; then
        TIMEOUT_CMD="gtimeout $TEST_TIMEOUT"
    else
        TIMEOUT_CMD=""
        log_warn "No timeout command available"
    fi
}

install_ci_dependencies() {
    log_info "Installing CI dependencies..."
    
    # Install coverage tools
    if [[ "$GENERATE_COVERAGE" == "true" ]]; then
        if ! command -v cargo-tarpaulin &> /dev/null; then
            log_info "Installing cargo-tarpaulin for coverage..."
            cargo install cargo-tarpaulin --quiet || log_warn "Failed to install cargo-tarpaulin"
        fi
        
        if ! command -v grcov &> /dev/null; then
            log_info "Installing grcov for coverage..."
            cargo install grcov --quiet || log_warn "Failed to install grcov"
        fi
    fi
    
    # Install other CI tools
    if ! command -v cargo-audit &> /dev/null; then
        log_info "Installing cargo-audit for security checks..."
        cargo install cargo-audit --quiet || log_warn "Failed to install cargo-audit"
    fi
    
    if ! command -v cargo-outdated &> /dev/null; then
        log_info "Installing cargo-outdated for dependency checks..."
        cargo install cargo-outdated --quiet || log_warn "Failed to install cargo-outdated"
    fi
}

# Run security and dependency checks
run_security_checks() {
    log "Running security and dependency checks..."
    
    local check_results=0
    
    # Audit dependencies for security vulnerabilities
    if command -v cargo-audit &> /dev/null; then
        log_info "Running security audit..."
        cargo audit --json > "$CI_RESULTS_DIR/security_audit.json" 2>&1 || {
            log_warn "Security audit found issues"
            check_results=1
        }
    fi
    
    # Check for outdated dependencies
    if command -v cargo-outdated &> /dev/null; then
        log_info "Checking for outdated dependencies..."
        cargo outdated --format json > "$CI_RESULTS_DIR/outdated_deps.json" 2>&1 || {
            log_warn "Found outdated dependencies"
        }
    fi
    
    # Lint code
    log_info "Running cargo clippy..."
    cargo clippy --all-targets --all-features -- -D warnings > "$CI_RESULTS_DIR/clippy.log" 2>&1 || {
        log_error "Clippy found issues"
        check_results=1
    }
    
    # Format check
    log_info "Checking code formatting..."
    cargo fmt --all -- --check > "$CI_RESULTS_DIR/fmt_check.log" 2>&1 || {
        log_error "Code formatting issues found"
        check_results=1
    }
    
    return $check_results
}

# Run unit tests with optimizations for CI
run_ci_unit_tests() {
    if [[ "$RUN_UNIT_TESTS" != "true" ]]; then
        return 0
    fi
    
    log "Running unit tests in CI mode..."
    
    local test_results=0
    local test_start=$(date +%s)
    
    # Configure test execution
    local test_args=()
    test_args+=("--verbose")
    test_args+=("--color=always")
    
    if [[ "$PARALLEL_JOBS" -gt 1 ]]; then
        test_args+=("--jobs=$PARALLEL_JOBS")
    fi
    
    if [[ "$QUICK_MODE" == "true" ]]; then
        test_args+=("--lib") # Only library tests, skip integration tests
    fi
    
    # Run tests with coverage if enabled
    if [[ "$GENERATE_COVERAGE" == "true" ]] && command -v cargo-tarpaulin &> /dev/null; then
        log_info "Running unit tests with coverage..."
        
        $TIMEOUT_CMD cargo tarpaulin \
            --out xml \
            --output-dir "$CI_RESULTS_DIR" \
            --timeout 120 \
            --jobs "$PARALLEL_JOBS" \
            --verbose \
            2>&1 | tee "$CI_RESULTS_DIR/unit_tests_coverage.log" || test_results=$?
            
        # Check coverage threshold
        if [[ -f "$CI_RESULTS_DIR/cobertura.xml" ]]; then
            check_coverage_threshold || test_results=$?
        fi
    else
        log_info "Running unit tests without coverage..."
        
        $TIMEOUT_CMD cargo test "${test_args[@]}" \
            2>&1 | tee "$CI_RESULTS_DIR/unit_tests.log" || test_results=$?
    fi
    
    local test_end=$(date +%s)
    local test_duration=$((test_end - test_start))
    
    if [[ $test_results -eq 0 ]]; then
        log "✅ Unit tests passed in ${test_duration}s"
    else
        log_error "❌ Unit tests failed in ${test_duration}s"
    fi
    
    return $test_results
}

# Check coverage threshold
check_coverage_threshold() {
    log_info "Checking coverage threshold ($COVERAGE_THRESHOLD%)..."
    
    if command -v jq &> /dev/null && [[ -f "$CI_RESULTS_DIR/cobertura.xml" ]]; then
        # Parse coverage from XML (simplified)
        local coverage=$(grep -o 'line-rate="[0-9.]*"' "$CI_RESULTS_DIR/cobertura.xml" | head -1 | cut -d'"' -f2)
        if [[ -n "$coverage" ]]; then
            local coverage_percent=$(echo "$coverage * 100" | bc -l | cut -d. -f1)
            
            log_info "Code coverage: ${coverage_percent}%"
            
            if [[ $coverage_percent -lt $COVERAGE_THRESHOLD ]]; then
                log_error "Coverage ${coverage_percent}% below threshold ${COVERAGE_THRESHOLD}%"
                return 1
            else
                log "✅ Coverage ${coverage_percent}% meets threshold ${COVERAGE_THRESHOLD}%"
            fi
        fi
    else
        log_warn "Could not parse coverage information"
    fi
    
    return 0
}

# Run integration tests optimized for CI
run_ci_integration_tests() {
    if [[ "$RUN_INTEGRATION_TESTS" != "true" ]]; then
        return 0
    fi
    
    log "Running integration tests in CI mode..."
    
    local test_results=0
    local test_start=$(date +%s)
    
    # Run integration tests with timeout
    $TIMEOUT_CMD cargo test \
        --test '*integration*' \
        --verbose \
        --color=always \
        --jobs="$PARALLEL_JOBS" \
        2>&1 | tee "$CI_RESULTS_DIR/integration_tests.log" || test_results=$?
    
    local test_end=$(date +%s)
    local test_duration=$((test_end - test_start))
    
    if [[ $test_results -eq 0 ]]; then
        log "✅ Integration tests passed in ${test_duration}s"
    else
        log_error "❌ Integration tests failed in ${test_duration}s"
    fi
    
    return $test_results
}

# Run performance tests (abbreviated for CI)
run_ci_performance_tests() {
    if [[ "$RUN_PERFORMANCE_TESTS" != "true" ]]; then
        return 0
    fi
    
    log "Running performance tests in CI mode..."
    
    local test_results=0
    local test_start=$(date +%s)
    
    # Run abbreviated performance tests
    local benchmarks=("ring_buffer_bench" "collectors_bench")
    
    for benchmark in "${benchmarks[@]}"; do
        log_info "Running $benchmark..."
        
        $TIMEOUT_CMD cargo bench \
            --bench "$benchmark" \
            -- --sample-size 10 \
            2>&1 | tee "$CI_RESULTS_DIR/${benchmark}.log" || test_results=$?
    done
    
    local test_end=$(date +%s)
    local test_duration=$((test_end - test_start))
    
    if [[ $test_results -eq 0 ]]; then
        log "✅ Performance tests passed in ${test_duration}s"
    else
        log_error "❌ Performance tests failed in ${test_duration}s"
    fi
    
    return $test_results
}

# Run Swift tests (macOS only)
run_ci_swift_tests() {
    if [[ "$RUN_SWIFT_TESTS" != "true" ]] || [[ "$OSTYPE" != "darwin"* ]]; then
        return 0
    fi
    
    log "Running Swift tests in CI mode..."
    
    local test_results=0
    local test_start=$(date +%s)
    
    # Run Swift tests with CI-specific settings
    cd "$PROJECT_ROOT/collectors"
    
    $TIMEOUT_CMD xcodebuild test \
        -project ChronicleCollectors.xcodeproj \
        -scheme ChronicleCollectors \
        -destination 'platform=macOS' \
        -enableCodeCoverage YES \
        -resultBundlePath "$CI_RESULTS_DIR/ChronicleCollectors.xcresult" \
        -quiet \
        2>&1 | tee "$CI_RESULTS_DIR/swift_tests.log" || test_results=$?
    
    cd "$SCRIPT_DIR"
    
    local test_end=$(date +%s)
    local test_duration=$((test_end - test_start))
    
    if [[ $test_results -eq 0 ]]; then
        log "✅ Swift tests passed in ${test_duration}s"
    else
        log_error "❌ Swift tests failed in ${test_duration}s"
    fi
    
    return $test_results
}

# Generate CI artifacts
generate_ci_artifacts() {
    log "Generating CI artifacts..."
    
    # Create test summary
    create_test_summary
    
    # Create coverage reports
    if [[ "$GENERATE_COVERAGE" == "true" ]]; then
        create_coverage_reports
    fi
    
    # Create performance summaries
    if [[ "$RUN_PERFORMANCE_TESTS" == "true" ]]; then
        create_performance_summary
    fi
    
    # Create artifact archive
    create_artifact_archive
}

create_test_summary() {
    local summary_file="$CI_RESULTS_DIR/test_summary.json"
    local end_time=$(date +%s)
    local start_time=$(stat -f %Y "$LOG_FILE" 2>/dev/null || stat -c %Y "$LOG_FILE" 2>/dev/null || echo $end_time)
    local duration=$((end_time - start_time))
    
    cat > "$summary_file" << EOF
{
    "ci_run": {
        "environment": "$CI_ENVIRONMENT",
        "branch": "$CI_BRANCH",
        "commit": "$CI_COMMIT",
        "timestamp": "$(date -Iseconds)",
        "duration_seconds": $duration,
        "quick_mode": $QUICK_MODE,
        "coverage_threshold": $COVERAGE_THRESHOLD
    },
    "test_results": {
        "unit_tests": "$(get_test_result "unit")",
        "integration_tests": "$(get_test_result "integration")",
        "performance_tests": "$(get_test_result "performance")",
        "swift_tests": "$(get_test_result "swift")",
        "security_checks": "$(get_test_result "security")"
    },
    "artifacts": {
        "logs_directory": "$CI_RESULTS_DIR",
        "coverage_report": "$(ls $CI_RESULTS_DIR/cobertura.xml 2>/dev/null || echo "not_generated")",
        "performance_reports": "$(ls $CI_RESULTS_DIR/*_bench.log 2>/dev/null | wc -l) files"
    }
}
EOF
    
    log_info "Test summary created: $summary_file"
}

create_coverage_reports() {
    if [[ -f "$CI_RESULTS_DIR/cobertura.xml" ]]; then
        log_info "Coverage report available: $CI_RESULTS_DIR/cobertura.xml"
        
        # Create HTML coverage report if possible
        if command -v grcov &> /dev/null; then
            log_info "Generating HTML coverage report..."
            grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing \
                -o "$CI_RESULTS_DIR/coverage_html/" 2>/dev/null || log_warn "Failed to generate HTML coverage"
        fi
    fi
}

create_performance_summary() {
    local perf_summary="$CI_RESULTS_DIR/performance_summary.txt"
    
    echo "Performance Test Summary - $(date)" > "$perf_summary"
    echo "=======================================" >> "$perf_summary"
    
    for log_file in "$CI_RESULTS_DIR"/*_bench.log; do
        if [[ -f "$log_file" ]]; then
            echo "" >> "$perf_summary"
            echo "=== $(basename "$log_file" .log) ===" >> "$perf_summary"
            grep -E "(time|ns/iter|MB/s)" "$log_file" | head -10 >> "$perf_summary" 2>/dev/null || true
        fi
    done
    
    log_info "Performance summary created: $perf_summary"
}

create_artifact_archive() {
    local archive_name="chronicle_ci_artifacts_$(date +%Y%m%d_%H%M%S).tar.gz"
    local archive_path="$CI_RESULTS_DIR/$archive_name"
    
    log_info "Creating artifact archive: $archive_name"
    
    tar -czf "$archive_path" -C "$CI_RESULTS_DIR" \
        --exclude="$archive_name" \
        --exclude="test_data" \
        . 2>/dev/null || log_warn "Failed to create artifact archive"
    
    if [[ -f "$archive_path" ]]; then
        log_info "Artifact archive created: $archive_path"
        log_info "Archive size: $(du -h "$archive_path" | cut -f1)"
    fi
}

get_test_result() {
    local test_type=$1
    # This would analyze the actual test results from log files
    # For now, return a placeholder
    echo "completed"
}

# Output CI-specific information
output_ci_info() {
    log "CI Test Information:"
    log_info "Environment: $CI_ENVIRONMENT"
    log_info "Branch: $CI_BRANCH"
    log_info "Commit: ${CI_COMMIT:0:8}"
    log_info "Quick Mode: $QUICK_MODE"
    log_info "Parallel Jobs: $PARALLEL_JOBS"
    log_info "Coverage Threshold: $COVERAGE_THRESHOLD%"
    log_info "Total Timeout: ${CI_TIMEOUT}s"
    log_info "Results Directory: $CI_RESULTS_DIR"
}

# Main CI execution function
main() {
    # Set up signal handlers for CI
    trap 'log_error "CI tests interrupted"; exit 1' INT TERM
    trap 'generate_ci_artifacts' EXIT
    
    # Parse arguments and configuration
    parse_args "$@"
    parse_ci_config
    
    # Setup CI environment
    setup_ci_environment
    
    # Output CI information
    output_ci_info
    
    local overall_result=0
    local start_time=$(date +%s)
    
    # Run security and dependency checks first
    run_security_checks || overall_result=$?
    
    # Run test suites
    run_ci_unit_tests || overall_result=$?
    run_ci_integration_tests || overall_result=$?
    run_ci_performance_tests || overall_result=$?
    run_ci_swift_tests || overall_result=$?
    
    local end_time=$(date +%s)
    local total_duration=$((end_time - start_time))
    
    # Check if we exceeded the timeout
    if [[ $total_duration -gt $CI_TIMEOUT ]]; then
        log_error "CI tests exceeded timeout of ${CI_TIMEOUT}s (took ${total_duration}s)"
        overall_result=1
    fi
    
    # Final summary
    if [[ $overall_result -eq 0 ]]; then
        log "✅ All CI tests completed successfully in ${total_duration}s!"
        log "📊 CI artifacts available in: $CI_RESULTS_DIR"
    else
        log_error "❌ CI tests failed. Check logs for details."
        log_error "📊 CI artifacts available in: $CI_RESULTS_DIR"
    fi
    
    return $overall_result
}

# Run main function
main "$@"