#!/bin/bash

# Chronicle Complete Test Suite Runner
# Runs all tests including unit, integration, and performance tests

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
PARALLEL=true
COVERAGE=false
FAIL_FAST=false
REPORT_FORMAT="text"
OUTPUT_DIR="$ROOT_DIR/test-results"
COMPONENTS=()
TEST_TYPES=()

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
Usage: $0 [OPTIONS]

Run complete Chronicle test suite.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -p, --parallel          Run tests in parallel (default)
    -s, --serial            Run tests serially
    -c, --coverage          Generate coverage reports
    -f, --fail-fast         Stop on first failure
    --output-dir DIR        Output directory for test results
    --format FORMAT         Report format (text, xml, json, html)
    --unit                  Run only unit tests
    --integration           Run only integration tests
    --performance           Run only performance tests
    --smoke                 Run only smoke tests
    --ring-buffer           Test only ring buffer component
    --packer                Test only packer component
    --collectors            Test only collectors component
    --cli                   Test only CLI component
    --ui                    Test only UI component
    --benchmarks            Test only benchmarks component

EXAMPLES:
    $0                      # Run all tests
    $0 --coverage           # Run all tests with coverage
    $0 --unit --parallel    # Run unit tests in parallel
    $0 --format xml         # Generate XML reports

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
            -p|--parallel)
                PARALLEL=true
                shift
                ;;
            -s|--serial)
                PARALLEL=false
                shift
                ;;
            -c|--coverage)
                COVERAGE=true
                shift
                ;;
            -f|--fail-fast)
                FAIL_FAST=true
                shift
                ;;
            --output-dir)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            --format)
                REPORT_FORMAT="$2"
                shift 2
                ;;
            --unit)
                TEST_TYPES+=("unit")
                shift
                ;;
            --integration)
                TEST_TYPES+=("integration")
                shift
                ;;
            --performance)
                TEST_TYPES+=("performance")
                shift
                ;;
            --smoke)
                TEST_TYPES+=("smoke")
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
            -*)
                error "Unknown option: $1"
                ;;
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done
    
    # If no test types specified, run all
    if [ ${#TEST_TYPES[@]} -eq 0 ]; then
        TEST_TYPES=("unit" "integration" "performance" "smoke")
    fi
    
    # If no components specified, test all
    if [ ${#COMPONENTS[@]} -eq 0 ]; then
        COMPONENTS=("ring-buffer" "packer" "collectors" "cli" "ui" "benchmarks")
    fi
    
    # Validate report format
    case $REPORT_FORMAT in
        "text"|"xml"|"json"|"html")
            ;;
        *)
            error "Invalid report format: $REPORT_FORMAT. Use: text, xml, json, html"
            ;;
    esac
}

# Setup test environment
setup_test_env() {
    log "Setting up test environment..."
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    # Set test environment variables
    export RUST_BACKTRACE=1
    export RUST_LOG=debug
    export CHRONICLE_TEST_MODE=1
    export CHRONICLE_TEST_OUTPUT_DIR="$OUTPUT_DIR"
    
    # Clean previous test results
    rm -rf "$OUTPUT_DIR"/*
    
    log "Test environment ready"
}

# Run unit tests
run_unit_tests() {
    if [[ ! " ${TEST_TYPES[*]} " =~ " unit " ]]; then
        return 0
    fi
    
    log "Running unit tests..."
    
    local unit_script="$SCRIPT_DIR/run_unit_tests.sh"
    local unit_flags=""
    
    if [ "$VERBOSE" = true ]; then
        unit_flags="$unit_flags --verbose"
    fi
    
    if [ "$COVERAGE" = true ]; then
        unit_flags="$unit_flags --coverage"
    fi
    
    if [ "$FAIL_FAST" = true ]; then
        unit_flags="$unit_flags --fail-fast"
    fi
    
    if [ "$PARALLEL" = true ]; then
        unit_flags="$unit_flags --parallel"
    else
        unit_flags="$unit_flags --serial"
    fi
    
    # Add component filters
    for component in "${COMPONENTS[@]}"; do
        unit_flags="$unit_flags --$component"
    done
    
    if ! "$unit_script" $unit_flags --output-dir "$OUTPUT_DIR/unit"; then
        if [ "$FAIL_FAST" = true ]; then
            error "Unit tests failed"
        else
            warn "Unit tests failed, continuing..."
            return 1
        fi
    fi
    
    log "Unit tests completed"
    return 0
}

# Run integration tests
run_integration_tests() {
    if [[ ! " ${TEST_TYPES[*]} " =~ " integration " ]]; then
        return 0
    fi
    
    log "Running integration tests..."
    
    local integration_script="$SCRIPT_DIR/run_integration_tests.sh"
    local integration_flags=""
    
    if [ "$VERBOSE" = true ]; then
        integration_flags="$integration_flags --verbose"
    fi
    
    if [ "$COVERAGE" = true ]; then
        integration_flags="$integration_flags --coverage"
    fi
    
    if [ "$FAIL_FAST" = true ]; then
        integration_flags="$integration_flags --fail-fast"
    fi
    
    if [ "$PARALLEL" = true ]; then
        integration_flags="$integration_flags --parallel"
    else
        integration_flags="$integration_flags --serial"
    fi
    
    # Add component filters
    for component in "${COMPONENTS[@]}"; do
        integration_flags="$integration_flags --$component"
    done
    
    if ! "$integration_script" $integration_flags --output-dir "$OUTPUT_DIR/integration"; then
        if [ "$FAIL_FAST" = true ]; then
            error "Integration tests failed"
        else
            warn "Integration tests failed, continuing..."
            return 1
        fi
    fi
    
    log "Integration tests completed"
    return 0
}

# Run performance tests
run_performance_tests() {
    if [[ ! " ${TEST_TYPES[*]} " =~ " performance " ]]; then
        return 0
    fi
    
    log "Running performance tests..."
    
    local performance_script="$SCRIPT_DIR/run_performance_tests.sh"
    local performance_flags=""
    
    if [ "$VERBOSE" = true ]; then
        performance_flags="$performance_flags --verbose"
    fi
    
    if [ "$FAIL_FAST" = true ]; then
        performance_flags="$performance_flags --fail-fast"
    fi
    
    # Add component filters
    for component in "${COMPONENTS[@]}"; do
        performance_flags="$performance_flags --$component"
    done
    
    if ! "$performance_script" $performance_flags --output-dir "$OUTPUT_DIR/performance"; then
        if [ "$FAIL_FAST" = true ]; then
            error "Performance tests failed"
        else
            warn "Performance tests failed, continuing..."
            return 1
        fi
    fi
    
    log "Performance tests completed"
    return 0
}

# Run smoke tests
run_smoke_tests() {
    if [[ ! " ${TEST_TYPES[*]} " =~ " smoke " ]]; then
        return 0
    fi
    
    log "Running smoke tests..."
    
    # Build smoke test list
    local smoke_tests=()
    
    # CLI smoke test
    if [[ " ${COMPONENTS[*]} " =~ " cli " ]]; then
        smoke_tests+=("cli_version" "cli_help")
    fi
    
    # Packer smoke test
    if [[ " ${COMPONENTS[*]} " =~ " packer " ]]; then
        smoke_tests+=("packer_help" "packer_config")
    fi
    
    # Ring buffer smoke test
    if [[ " ${COMPONENTS[*]} " =~ " ring-buffer " ]]; then
        smoke_tests+=("ring_buffer_basic")
    fi
    
    local smoke_failed=false
    
    for test in "${smoke_tests[@]}"; do
        if ! run_smoke_test "$test"; then
            smoke_failed=true
            if [ "$FAIL_FAST" = true ]; then
                error "Smoke test failed: $test"
            fi
        fi
    done
    
    if [ "$smoke_failed" = true ]; then
        warn "Some smoke tests failed"
        return 1
    fi
    
    log "Smoke tests completed"
    return 0
}

# Run individual smoke test
run_smoke_test() {
    local test_name="$1"
    
    info "Running smoke test: $test_name"
    
    case $test_name in
        "cli_version")
            if [ -f "$ROOT_DIR/cli/target/debug/chronicle" ]; then
                "$ROOT_DIR/cli/target/debug/chronicle" --version
            elif [ -f "$ROOT_DIR/cli/target/release/chronicle" ]; then
                "$ROOT_DIR/cli/target/release/chronicle" --version
            else
                warn "CLI binary not found for smoke test"
                return 1
            fi
            ;;
        "cli_help")
            if [ -f "$ROOT_DIR/cli/target/debug/chronicle" ]; then
                "$ROOT_DIR/cli/target/debug/chronicle" --help > /dev/null
            elif [ -f "$ROOT_DIR/cli/target/release/chronicle" ]; then
                "$ROOT_DIR/cli/target/release/chronicle" --help > /dev/null
            else
                warn "CLI binary not found for smoke test"
                return 1
            fi
            ;;
        "packer_help")
            if [ -f "$ROOT_DIR/packer/target/debug/chronicle-packer" ]; then
                "$ROOT_DIR/packer/target/debug/chronicle-packer" --help > /dev/null
            elif [ -f "$ROOT_DIR/packer/target/release/chronicle-packer" ]; then
                "$ROOT_DIR/packer/target/release/chronicle-packer" --help > /dev/null
            else
                warn "Packer binary not found for smoke test"
                return 1
            fi
            ;;
        "ring_buffer_basic")
            if [ -f "$ROOT_DIR/ring-buffer/test_ring_buffer" ]; then
                cd "$ROOT_DIR/ring-buffer"
                ./test_ring_buffer
            else
                warn "Ring buffer test not found for smoke test"
                return 1
            fi
            ;;
        *)
            warn "Unknown smoke test: $test_name"
            return 1
            ;;
    esac
    
    return 0
}

# Run all test types
run_all_test_types() {
    log "Running test types: ${TEST_TYPES[*]}"
    
    local test_results=()
    
    if [ "$PARALLEL" = true ]; then
        # Run test types in parallel
        local pids=()
        
        (run_unit_tests && echo "unit:success" || echo "unit:failure") > "$OUTPUT_DIR/unit_result" &
        pids+=($!)
        
        (run_integration_tests && echo "integration:success" || echo "integration:failure") > "$OUTPUT_DIR/integration_result" &
        pids+=($!)
        
        (run_performance_tests && echo "performance:success" || echo "performance:failure") > "$OUTPUT_DIR/performance_result" &
        pids+=($!)
        
        (run_smoke_tests && echo "smoke:success" || echo "smoke:failure") > "$OUTPUT_DIR/smoke_result" &
        pids+=($!)
        
        # Wait for all to complete
        for pid in "${pids[@]}"; do
            wait "$pid"
        done
        
        # Collect results
        for result_file in "$OUTPUT_DIR"/*_result; do
            if [ -f "$result_file" ]; then
                test_results+=($(cat "$result_file"))
            fi
        done
    else
        # Run test types serially
        if run_unit_tests; then
            test_results+=("unit:success")
        else
            test_results+=("unit:failure")
        fi
        
        if run_integration_tests; then
            test_results+=("integration:success")
        else
            test_results+=("integration:failure")
        fi
        
        if run_performance_tests; then
            test_results+=("performance:success")
        else
            test_results+=("performance:failure")
        fi
        
        if run_smoke_tests; then
            test_results+=("smoke:success")
        else
            test_results+=("smoke:failure")
        fi
    fi
    
    # Check results
    local failed_tests=()
    for result in "${test_results[@]}"; do
        local test_type="${result%:*}"
        local test_status="${result#*:}"
        
        if [ "$test_status" = "failure" ]; then
            failed_tests+=("$test_type")
        fi
    done
    
    if [ ${#failed_tests[@]} -gt 0 ]; then
        error "Failed test types: ${failed_tests[*]}"
    fi
}

# Generate comprehensive test report
generate_comprehensive_report() {
    log "Generating comprehensive test report..."
    
    local report_file="$OUTPUT_DIR/test-report.$REPORT_FORMAT"
    
    case $REPORT_FORMAT in
        "text")
            generate_text_report "$report_file"
            ;;
        "xml")
            generate_xml_report "$report_file"
            ;;
        "json")
            generate_json_report "$report_file"
            ;;
        "html")
            generate_html_report "$report_file"
            ;;
    esac
    
    log "Test report generated: $report_file"
}

# Generate text report
generate_text_report() {
    local report_file="$1"
    
    cat > "$report_file" << EOF
Chronicle Complete Test Suite Report
Generated: $(date)

Configuration:
  Test Types: ${TEST_TYPES[*]}
  Components: ${COMPONENTS[*]}
  Parallel Execution: $PARALLEL
  Coverage Enabled: $COVERAGE
  Fail Fast: $FAIL_FAST

Test Results Summary:
EOF
    
    # Add test results from individual report files
    for test_type in "${TEST_TYPES[@]}"; do
        local test_dir="$OUTPUT_DIR/$test_type"
        if [ -d "$test_dir" ]; then
            echo "" >> "$report_file"
            echo "$test_type Tests:" >> "$report_file"
            if [ -f "$test_dir/summary.txt" ]; then
                cat "$test_dir/summary.txt" >> "$report_file"
            else
                echo "  No summary available" >> "$report_file"
            fi
        fi
    done
    
    # Add coverage summary if available
    if [ "$COVERAGE" = true ]; then
        echo "" >> "$report_file"
        echo "Coverage Summary:" >> "$report_file"
        
        for component in "${COMPONENTS[@]}"; do
            local coverage_file="$OUTPUT_DIR/unit/$component/coverage/coverage.txt"
            if [ -f "$coverage_file" ]; then
                echo "  $component: $(cat "$coverage_file")" >> "$report_file"
            fi
        done
    fi
}

# Generate XML report (JUnit format)
generate_xml_report() {
    local report_file="$1"
    
    cat > "$report_file" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="Chronicle Test Suite" tests="0" failures="0" time="0">
EOF
    
    # Add test suites from individual XML reports
    for test_type in "${TEST_TYPES[@]}"; do
        local test_dir="$OUTPUT_DIR/$test_type"
        if [ -f "$test_dir/junit.xml" ]; then
            # Extract testsuite elements and add them
            grep "<testsuite" "$test_dir/junit.xml" >> "$report_file" || true
        fi
    done
    
    echo "</testsuites>" >> "$report_file"
}

# Generate JSON report
generate_json_report() {
    local report_file="$1"
    
    cat > "$report_file" << EOF
{
  "test_suite": "Chronicle Complete Test Suite",
  "generated": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "configuration": {
    "test_types": [$(printf '"%s",' "${TEST_TYPES[@]}" | sed 's/,$//')]",
    "components": [$(printf '"%s",' "${COMPONENTS[@]}" | sed 's/,$//')]",
    "parallel": $PARALLEL,
    "coverage": $COVERAGE,
    "fail_fast": $FAIL_FAST
  },
  "results": {
EOF
    
    local first=true
    for test_type in "${TEST_TYPES[@]}"; do
        if [ "$first" = false ]; then
            echo "," >> "$report_file"
        fi
        first=false
        
        echo "    \"$test_type\": {" >> "$report_file"
        
        local test_dir="$OUTPUT_DIR/$test_type"
        if [ -f "$test_dir/results.json" ]; then
            cat "$test_dir/results.json" >> "$report_file"
        else
            echo "      \"status\": \"unknown\"" >> "$report_file"
        fi
        
        echo "    }" >> "$report_file"
    done
    
    cat >> "$report_file" << EOF
  }
}
EOF
}

# Generate HTML report
generate_html_report() {
    local report_file="$1"
    
    cat > "$report_file" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>Chronicle Test Suite Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { border-bottom: 2px solid #333; padding-bottom: 20px; }
        .section { margin: 20px 0; }
        .success { color: green; }
        .failure { color: red; }
        .warning { color: orange; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Chronicle Test Suite Report</h1>
        <p>Generated: $(date)</p>
    </div>
    
    <div class="section">
        <h2>Configuration</h2>
        <ul>
            <li>Test Types: ${TEST_TYPES[*]}</li>
            <li>Components: ${COMPONENTS[*]}</li>
            <li>Parallel Execution: $PARALLEL</li>
            <li>Coverage Enabled: $COVERAGE</li>
        </ul>
    </div>
    
    <div class="section">
        <h2>Test Results</h2>
        <table>
            <tr><th>Test Type</th><th>Status</th><th>Details</th></tr>
EOF
    
    for test_type in "${TEST_TYPES[@]}"; do
        echo "            <tr><td>$test_type</td><td>Results</td><td>Details</td></tr>" >> "$report_file"
    done
    
    cat >> "$report_file" << EOF
        </table>
    </div>
</body>
</html>
EOF
}

# Cleanup test artifacts
cleanup_test_artifacts() {
    log "Cleaning up test artifacts..."
    
    # Remove temporary files but keep reports
    find "$OUTPUT_DIR" -name "*.tmp" -delete 2>/dev/null || true
    find "$OUTPUT_DIR" -name "*_result" -delete 2>/dev/null || true
    
    log "Cleanup complete"
}

# Main test function
main() {
    log "Starting Chronicle complete test suite..."
    
    parse_args "$@"
    
    info "Test configuration:"
    info "  Test Types: ${TEST_TYPES[*]}"
    info "  Components: ${COMPONENTS[*]}"
    info "  Parallel: $PARALLEL"
    info "  Coverage: $COVERAGE"
    info "  Fail Fast: $FAIL_FAST"
    info "  Output Directory: $OUTPUT_DIR"
    info "  Report Format: $REPORT_FORMAT"
    
    setup_test_env
    run_all_test_types
    generate_comprehensive_report
    cleanup_test_artifacts
    
    log "Complete test suite finished!"
    info "Test reports available in: $OUTPUT_DIR"
}

# Run main function
main "$@"