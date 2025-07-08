#!/bin/bash
set -euo pipefail

# Chronicle Test Suite Runner
# Comprehensive testing script for the Chronicle project

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_RESULTS_DIR="$SCRIPT_DIR/results"
LOG_FILE="$TEST_RESULTS_DIR/test_run_$(date +%Y%m%d_%H%M%S).log"

# Test categories
RUN_UNIT_TESTS=true
RUN_INTEGRATION_TESTS=true
RUN_PERFORMANCE_TESTS=true
RUN_STRESS_TESTS=false
RUN_SWIFT_TESTS=true
GENERATE_COVERAGE=true
GENERATE_REPORT=true

# Performance thresholds
MAX_MEMORY_MB=1000
MAX_CPU_PERCENT=80
MIN_THROUGHPUT_OPS_PER_SEC=1000

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --unit-only)
                RUN_INTEGRATION_TESTS=false
                RUN_PERFORMANCE_TESTS=false
                RUN_STRESS_TESTS=false
                RUN_SWIFT_TESTS=false
                shift
                ;;
            --integration-only)
                RUN_UNIT_TESTS=false
                RUN_PERFORMANCE_TESTS=false
                RUN_STRESS_TESTS=false
                RUN_SWIFT_TESTS=false
                shift
                ;;
            --performance-only)
                RUN_UNIT_TESTS=false
                RUN_INTEGRATION_TESTS=false
                RUN_STRESS_TESTS=false
                RUN_SWIFT_TESTS=false
                shift
                ;;
            --stress)
                RUN_STRESS_TESTS=true
                shift
                ;;
            --swift-only)
                RUN_UNIT_TESTS=false
                RUN_INTEGRATION_TESTS=false
                RUN_PERFORMANCE_TESTS=false
                RUN_STRESS_TESTS=false
                shift
                ;;
            --no-coverage)
                GENERATE_COVERAGE=false
                shift
                ;;
            --no-report)
                GENERATE_REPORT=false
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

show_help() {
    cat << EOF
Chronicle Test Suite Runner

Usage: $0 [OPTIONS]

Options:
    --unit-only         Run only unit tests
    --integration-only  Run only integration tests
    --performance-only  Run only performance tests
    --stress           Enable stress tests (disabled by default)
    --swift-only       Run only Swift tests
    --no-coverage      Skip coverage generation
    --no-report        Skip report generation
    --help             Show this help message

Examples:
    $0                    # Run all tests except stress tests
    $0 --unit-only        # Run only unit tests
    $0 --stress           # Run all tests including stress tests
    $0 --swift-only       # Run only Swift tests
EOF
}

# Logging functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}" | tee -a "$LOG_FILE"
}

log_warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}" | tee -a "$LOG_FILE"
}

log_info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}" | tee -a "$LOG_FILE"
}

# Setup test environment
setup_test_environment() {
    log "Setting up test environment..."
    
    # Create results directory
    mkdir -p "$TEST_RESULTS_DIR"
    
    # Initialize log file
    echo "Chronicle Test Suite Run - $(date)" > "$LOG_FILE"
    echo "=======================================" >> "$LOG_FILE"
    
    # Check dependencies
    check_dependencies
    
    # Set environment variables
    export RUST_LOG=debug
    export RUST_BACKTRACE=1
    export CHRONICLE_TEST_MODE=1
    export CHRONICLE_TEST_DATA_DIR="$TEST_RESULTS_DIR/test_data"
    
    # Create test data directory
    mkdir -p "$CHRONICLE_TEST_DATA_DIR"
    
    # Check system resources
    check_system_resources
}

check_dependencies() {
    log_info "Checking dependencies..."
    
    # Check Rust
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Please install Rust."
        exit 1
    fi
    
    # Check Swift (for macOS)
    if [[ "$OSTYPE" == "darwin"* ]] && ! command -v swift &> /dev/null; then
        log_error "Swift not found. Please install Xcode."
        exit 1
    fi
    
    # Check system tools
    local required_tools=("jq" "bc")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_warn "$tool not found. Some features may not work."
        fi
    done
    
    log_info "Dependencies check completed."
}

check_system_resources() {
    log_info "Checking system resources..."
    
    # Check available memory
    if [[ "$OSTYPE" == "darwin"* ]]; then
        local total_memory_kb=$(sysctl -n hw.memsize)
        local total_memory_mb=$((total_memory_kb / 1024 / 1024))
    else
        local total_memory_mb=$(grep MemTotal /proc/meminfo | awk '{print int($2/1024)}')
    fi
    
    if [[ $total_memory_mb -lt 4096 ]]; then
        log_warn "Low memory detected (${total_memory_mb}MB). Some tests may fail."
    fi
    
    # Check available disk space
    local available_space_mb=$(df "$SCRIPT_DIR" | tail -1 | awk '{print int($4/1024)}')
    if [[ $available_space_mb -lt 1024 ]]; then
        log_warn "Low disk space detected (${available_space_mb}MB). Some tests may fail."
    fi
    
    log_info "System resources check completed."
}

# Run unit tests
run_unit_tests() {
    if [[ "$RUN_UNIT_TESTS" != "true" ]]; then
        return 0
    fi
    
    log "Running unit tests..."
    
    local test_results=0
    
    # Run Rust unit tests for each component
    local components=("packer" "cli")
    for component in "${components[@]}"; do
        log_info "Running unit tests for $component..."
        
        cd "$PROJECT_ROOT/$component"
        
        if [[ "$GENERATE_COVERAGE" == "true" ]]; then
            # Run with coverage
            if command -v cargo-tarpaulin &> /dev/null; then
                cargo tarpaulin --out xml --output-dir "$TEST_RESULTS_DIR" --timeout 120 || test_results=$?
            else
                log_warn "cargo-tarpaulin not found. Running tests without coverage."
                cargo test --verbose || test_results=$?
            fi
        else
            cargo test --verbose || test_results=$?
        fi
        
        cd "$SCRIPT_DIR"
    done
    
    # Run tests for test suite itself
    log_info "Running tests for test suite..."
    cargo test --verbose || test_results=$?
    
    if [[ $test_results -eq 0 ]]; then
        log "Unit tests completed successfully."
    else
        log_error "Unit tests failed with exit code $test_results."
    fi
    
    return $test_results
}

# Run integration tests
run_integration_tests() {
    if [[ "$RUN_INTEGRATION_TESTS" != "true" ]]; then
        return 0
    fi
    
    log "Running integration tests..."
    
    local test_results=0
    
    # Run integration tests
    cargo test --test '*integration*' --verbose || test_results=$?
    
    if [[ $test_results -eq 0 ]]; then
        log "Integration tests completed successfully."
    else
        log_error "Integration tests failed with exit code $test_results."
    fi
    
    return $test_results
}

# Run performance tests
run_performance_tests() {
    if [[ "$RUN_PERFORMANCE_TESTS" != "true" ]]; then
        return 0
    fi
    
    log "Running performance tests..."
    
    local test_results=0
    
    # Run benchmark tests
    cargo bench --bench ring_buffer_bench 2>&1 | tee "$TEST_RESULTS_DIR/ring_buffer_bench.log" || test_results=$?
    cargo bench --bench collectors_bench 2>&1 | tee "$TEST_RESULTS_DIR/collectors_bench.log" || test_results=$?
    cargo bench --bench packer_bench 2>&1 | tee "$TEST_RESULTS_DIR/packer_bench.log" || test_results=$?
    cargo bench --bench search_bench 2>&1 | tee "$TEST_RESULTS_DIR/search_bench.log" || test_results=$?
    
    # Analyze benchmark results
    analyze_benchmark_results
    
    if [[ $test_results -eq 0 ]]; then
        log "Performance tests completed successfully."
    else
        log_error "Performance tests failed with exit code $test_results."
    fi
    
    return $test_results
}

# Run stress tests
run_stress_tests() {
    if [[ "$RUN_STRESS_TESTS" != "true" ]]; then
        return 0
    fi
    
    log "Running stress tests..."
    log_warn "Stress tests may take a long time and consume significant resources."
    
    local test_results=0
    
    # Monitor system resources during stress tests
    monitor_resources &
    local monitor_pid=$!
    
    # Run stress tests
    cargo bench --bench stress_test 2>&1 | tee "$TEST_RESULTS_DIR/stress_test.log" || test_results=$?
    
    # Stop resource monitoring
    kill $monitor_pid 2>/dev/null || true
    
    if [[ $test_results -eq 0 ]]; then
        log "Stress tests completed successfully."
    else
        log_error "Stress tests failed with exit code $test_results."
    fi
    
    return $test_results
}

# Run Swift tests
run_swift_tests() {
    if [[ "$RUN_SWIFT_TESTS" != "true" ]] || [[ "$OSTYPE" != "darwin"* ]]; then
        return 0
    fi
    
    log "Running Swift tests..."
    
    local test_results=0
    
    # Run Swift tests for collectors
    cd "$PROJECT_ROOT/collectors"
    
    xcodebuild test \
        -project ChronicleCollectors.xcodeproj \
        -scheme ChronicleCollectors \
        -destination 'platform=macOS' \
        -resultBundlePath "$TEST_RESULTS_DIR/ChronicleCollectors.xcresult" \
        2>&1 | tee "$TEST_RESULTS_DIR/swift_tests.log" || test_results=$?
    
    # Run Swift tests for UI
    cd "$PROJECT_ROOT/ui"
    
    xcodebuild test \
        -project ChronicleUI.xcodeproj \
        -scheme ChronicleUI \
        -destination 'platform=macOS' \
        -resultBundlePath "$TEST_RESULTS_DIR/ChronicleUI.xcresult" \
        2>&1 | tee -a "$TEST_RESULTS_DIR/swift_tests.log" || test_results=$?
    
    cd "$SCRIPT_DIR"
    
    if [[ $test_results -eq 0 ]]; then
        log "Swift tests completed successfully."
    else
        log_error "Swift tests failed with exit code $test_results."
    fi
    
    return $test_results
}

# Monitor system resources
monitor_resources() {
    local monitor_interval=5
    local resource_log="$TEST_RESULTS_DIR/resource_usage.log"
    
    echo "timestamp,memory_mb,cpu_percent,disk_io_mb" > "$resource_log"
    
    while true; do
        local timestamp=$(date +%s)
        
        # Get memory usage
        if [[ "$OSTYPE" == "darwin"* ]]; then
            local memory_mb=$(ps -A -o rss | awk '{sum+=$1} END {print int(sum/1024)}')
            local cpu_percent=$(ps -A -o %cpu | awk '{sum+=$1} END {print sum}')
        else
            local memory_mb=$(free -m | awk 'NR==2{print $3}')
            local cpu_percent=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//')
        fi
        
        # Get disk I/O (simplified)
        local disk_io_mb=0
        
        echo "$timestamp,$memory_mb,$cpu_percent,$disk_io_mb" >> "$resource_log"
        
        sleep $monitor_interval
    done
}

# Analyze benchmark results
analyze_benchmark_results() {
    log_info "Analyzing benchmark results..."
    
    local benchmark_files=("$TEST_RESULTS_DIR"/*_bench.log)
    local analysis_file="$TEST_RESULTS_DIR/benchmark_analysis.txt"
    
    echo "Benchmark Analysis - $(date)" > "$analysis_file"
    echo "=================================" >> "$analysis_file"
    
    for file in "${benchmark_files[@]}"; do
        if [[ -f "$file" ]]; then
            local benchmark_name=$(basename "$file" .log)
            echo "" >> "$analysis_file"
            echo "=== $benchmark_name ===" >> "$analysis_file"
            
            # Extract key metrics (this would be more sophisticated in practice)
            grep -E "(time:|throughput:|memory:)" "$file" >> "$analysis_file" 2>/dev/null || true
        fi
    done
    
    log_info "Benchmark analysis saved to $analysis_file"
}

# Generate test report
generate_test_report() {
    if [[ "$GENERATE_REPORT" != "true" ]]; then
        return 0
    fi
    
    log "Generating test report..."
    
    local report_file="$TEST_RESULTS_DIR/test_report.html"
    local summary_file="$TEST_RESULTS_DIR/test_summary.json"
    
    # Generate JSON summary
    generate_json_summary > "$summary_file"
    
    # Generate HTML report
    generate_html_report > "$report_file"
    
    log "Test report generated: $report_file"
    log "Test summary generated: $summary_file"
}

generate_json_summary() {
    local end_time=$(date +%s)
    local start_time=$(stat -f %Y "$LOG_FILE" 2>/dev/null || stat -c %Y "$LOG_FILE" 2>/dev/null || echo $end_time)
    local duration=$((end_time - start_time))
    
    cat << EOF
{
    "test_run": {
        "timestamp": "$(date -Iseconds)",
        "duration_seconds": $duration,
        "environment": {
            "os": "$OSTYPE",
            "arch": "$(uname -m)",
            "rust_version": "$(rustc --version)",
            "test_suite_version": "1.0.0"
        },
        "configuration": {
            "unit_tests": $RUN_UNIT_TESTS,
            "integration_tests": $RUN_INTEGRATION_TESTS,
            "performance_tests": $RUN_PERFORMANCE_TESTS,
            "stress_tests": $RUN_STRESS_TESTS,
            "swift_tests": $RUN_SWIFT_TESTS,
            "coverage_enabled": $GENERATE_COVERAGE
        },
        "results": {
            "overall_status": "$(get_overall_status)",
            "details": "See individual test logs for detailed results"
        }
    }
}
EOF
}

generate_html_report() {
    cat << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Chronicle Test Suite Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background: #f0f0f0; padding: 20px; border-radius: 5px; }
        .section { margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 5px; }
        .success { background: #d4edda; border-color: #c3e6cb; }
        .warning { background: #fff3cd; border-color: #ffeaa7; }
        .error { background: #f8d7da; border-color: #f5c6cb; }
        .metric { display: inline-block; margin: 10px; padding: 10px; background: #f8f9fa; border-radius: 3px; }
        pre { background: #f8f9fa; padding: 10px; border-radius: 3px; overflow-x: auto; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Chronicle Test Suite Report</h1>
        <p>Generated on: $(date)</p>
        <p>Total Duration: $(get_test_duration)</p>
    </div>
    
    <div class="section">
        <h2>Test Summary</h2>
        <div class="metric">Unit Tests: $(get_test_status "unit")</div>
        <div class="metric">Integration Tests: $(get_test_status "integration")</div>
        <div class="metric">Performance Tests: $(get_test_status "performance")</div>
        <div class="metric">Swift Tests: $(get_test_status "swift")</div>
    </div>
    
    <div class="section">
        <h2>System Information</h2>
        <pre>
OS: $OSTYPE
Architecture: $(uname -m)
Rust Version: $(rustc --version 2>/dev/null || echo "Not available")
Available Memory: $(get_available_memory)
        </pre>
    </div>
    
    <div class="section">
        <h2>Test Logs</h2>
        <p>Detailed test logs are available in the results directory:</p>
        <ul>
            <li><a href="test_run_*.log">Main test log</a></li>
            <li><a href="*_bench.log">Benchmark results</a></li>
            <li><a href="swift_tests.log">Swift test results</a></li>
        </ul>
    </div>
</body>
</html>
EOF
}

get_overall_status() {
    # This would analyze all test results and return overall status
    echo "completed"
}

get_test_duration() {
    echo "$(grep -E "Duration|Time" "$LOG_FILE" | tail -1 || echo "Unknown")"
}

get_test_status() {
    local test_type=$1
    # This would check the actual test results
    echo "completed"
}

get_available_memory() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "$(system_profiler SPHardwareDataType | grep Memory | awk '{print $2 $3}')"
    else
        echo "$(free -h | grep Mem | awk '{print $2}')"
    fi
}

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment..."
    
    # Kill any background processes
    jobs -p | xargs kill 2>/dev/null || true
    
    # Clean up temporary files
    if [[ -d "$CHRONICLE_TEST_DATA_DIR" ]]; then
        rm -rf "$CHRONICLE_TEST_DATA_DIR"
    fi
    
    log_info "Cleanup completed."
}

# Main execution
main() {
    # Parse command line arguments
    parse_args "$@"
    
    # Setup signal handlers
    trap cleanup EXIT
    trap 'log_error "Test run interrupted"; exit 1' INT TERM
    
    # Setup test environment
    setup_test_environment
    
    log "Starting Chronicle test suite..."
    log_info "Configuration: Unit=$RUN_UNIT_TESTS, Integration=$RUN_INTEGRATION_TESTS, Performance=$RUN_PERFORMANCE_TESTS, Stress=$RUN_STRESS_TESTS, Swift=$RUN_SWIFT_TESTS"
    
    local overall_result=0
    local start_time=$(date +%s)
    
    # Run test suites
    run_unit_tests || overall_result=$?
    run_integration_tests || overall_result=$?
    run_performance_tests || overall_result=$?
    run_stress_tests || overall_result=$?
    run_swift_tests || overall_result=$?
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    # Generate reports
    generate_test_report
    
    # Final summary
    if [[ $overall_result -eq 0 ]]; then
        log "✅ All tests completed successfully in ${duration}s!"
        log "📊 Test results available in: $TEST_RESULTS_DIR"
    else
        log_error "❌ Some tests failed. Check logs for details."
        log_error "📊 Test results available in: $TEST_RESULTS_DIR"
    fi
    
    return $overall_result
}

# Run main function with all arguments
main "$@"