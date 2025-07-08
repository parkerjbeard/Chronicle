#!/bin/bash

# Chronicle Unit Tests Runner
# Runs unit tests for all Chronicle components

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
OUTPUT_DIR="$ROOT_DIR/test-results/unit"
COMPONENTS=()
FILTER=""

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

Run Chronicle unit tests.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -p, --parallel          Run tests in parallel (default)
    -s, --serial            Run tests serially
    -c, --coverage          Generate coverage reports
    -f, --fail-fast         Stop on first failure
    --output-dir DIR        Output directory for test results
    --filter PATTERN        Filter tests by pattern
    --ring-buffer           Test only ring buffer component
    --packer                Test only packer component
    --collectors            Test only collectors component
    --cli                   Test only CLI component
    --ui                    Test only UI component
    --benchmarks            Test only benchmarks component

EXAMPLES:
    $0                      # Run all unit tests
    $0 --coverage           # Run with coverage
    $0 --filter "test_ring" # Run tests matching pattern
    $0 --cli --packer       # Test only CLI and packer

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
            --filter)
                FILTER="$2"
                shift 2
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
    
    # If no components specified, test all
    if [ ${#COMPONENTS[@]} -eq 0 ]; then
        COMPONENTS=("ring-buffer" "packer" "collectors" "cli" "ui" "benchmarks")
    fi
}

# Setup unit test environment
setup_unit_test_env() {
    log "Setting up unit test environment..."
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    # Set environment variables
    export RUST_BACKTRACE=1
    export RUST_LOG=warn  # Less verbose for unit tests
    export CHRONICLE_UNIT_TEST=1
    
    # Clean previous results
    rm -rf "$OUTPUT_DIR"/*
    
    log "Unit test environment ready"
}

# Run ring buffer unit tests
run_ring_buffer_unit_tests() {
    log "Running ring buffer unit tests..."
    
    local component_dir="$OUTPUT_DIR/ring-buffer"
    mkdir -p "$component_dir"
    
    cd "$ROOT_DIR/ring-buffer"
    
    # Build and run C unit tests
    local make_flags="test"
    if [ "$VERBOSE" = true ]; then
        make_flags="$make_flags VERBOSE=1"
    fi
    
    if ! make $make_flags > "$component_dir/build.log" 2>&1; then
        error "Ring buffer unit test build failed. Check $component_dir/build.log"
    fi
    
    # Run the test executable
    if [ -f "./test_ring_buffer" ]; then
        local test_output="$component_dir/test_output.txt"
        
        if ./test_ring_buffer > "$test_output" 2>&1; then
            log "Ring buffer unit tests passed"
            echo "PASSED" > "$component_dir/status.txt"
        else
            warn "Ring buffer unit tests failed"
            echo "FAILED" > "$component_dir/status.txt"
            
            if [ "$FAIL_FAST" = true ]; then
                error "Ring buffer unit tests failed"
            fi
            return 1
        fi
    else
        error "Ring buffer test executable not found"
    fi
    
    return 0
}

# Run Rust unit tests for a component
run_rust_unit_tests() {
    local component=$1
    local component_dir="$OUTPUT_DIR/$component"
    
    log "Running Rust unit tests for $component..."
    
    mkdir -p "$component_dir"
    
    if [ ! -d "$ROOT_DIR/$component" ]; then
        warn "Component directory not found: $component"
        return 0
    fi
    
    cd "$ROOT_DIR/$component"
    
    local cargo_flags="test --lib"
    if [ "$VERBOSE" = true ]; then
        cargo_flags="$cargo_flags --verbose"
    fi
    
    if [ -n "$FILTER" ]; then
        cargo_flags="$cargo_flags $FILTER"
    fi
    
    # Set up coverage if requested
    if [ "$COVERAGE" = true ]; then
        local coverage_dir="$component_dir/coverage"
        mkdir -p "$coverage_dir"
        
        # Use tarpaulin for coverage
        if command -v cargo-tarpaulin &> /dev/null; then
            local tarpaulin_flags="--out Html --output-dir $coverage_dir"
            if [ "$VERBOSE" = true ]; then
                tarpaulin_flags="$tarpaulin_flags --verbose"
            fi
            
            if [ -n "$FILTER" ]; then
                tarpaulin_flags="$tarpaulin_flags --test-filter $FILTER"
            fi
            
            if cargo tarpaulin $tarpaulin_flags > "$component_dir/coverage.log" 2>&1; then
                log "$component coverage report generated"
            else
                warn "$component coverage generation failed"
            fi
        else
            warn "cargo-tarpaulin not available for coverage"
        fi
    fi
    
    # Run the tests
    local test_output="$component_dir/test_output.txt"
    local junit_output="$component_dir/junit.xml"
    
    # Add JUnit output if possible
    if cargo test --help | grep -q -- --format; then
        cargo_flags="$cargo_flags --format pretty"
    fi
    
    if cargo $cargo_flags > "$test_output" 2>&1; then
        log "$component unit tests passed"
        echo "PASSED" > "$component_dir/status.txt"
        
        # Extract test statistics
        local test_count=$(grep -E "test result:|running [0-9]+ tests" "$test_output" | tail -1 || echo "unknown")
        echo "$test_count" > "$component_dir/stats.txt"
    else
        warn "$component unit tests failed"
        echo "FAILED" > "$component_dir/status.txt"
        
        if [ "$FAIL_FAST" = true ]; then
            error "$component unit tests failed"
        fi
        return 1
    fi
    
    return 0
}

# Run Swift unit tests for a component
run_swift_unit_tests() {
    local component=$1
    local component_dir="$OUTPUT_DIR/$component"
    
    log "Running Swift unit tests for $component..."
    
    mkdir -p "$component_dir"
    
    cd "$ROOT_DIR"
    
    local workspace_scheme=""
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
    
    local xcode_flags="test -workspace Chronicle.xcworkspace -scheme $workspace_scheme"
    xcode_flags="$xcode_flags -configuration Debug"
    xcode_flags="$xcode_flags -derivedDataPath $component_dir/derived_data"
    
    if [ "$VERBOSE" = false ]; then
        xcode_flags="$xcode_flags -quiet"
    fi
    
    # Add test filter if specified
    if [ -n "$FILTER" ]; then
        xcode_flags="$xcode_flags -only-testing:$workspace_scheme/$FILTER"
    fi
    
    # Generate coverage if requested
    if [ "$COVERAGE" = true ]; then
        xcode_flags="$xcode_flags -enableCodeCoverage YES"
    fi
    
    local test_output="$component_dir/test_output.txt"
    
    if xcodebuild $xcode_flags > "$test_output" 2>&1; then
        log "$component Swift unit tests passed"
        echo "PASSED" > "$component_dir/status.txt"
        
        # Extract test results
        if [ "$COVERAGE" = true ]; then
            # Generate coverage report
            local coverage_dir="$component_dir/coverage"
            mkdir -p "$coverage_dir"
            
            # Export coverage data (simplified)
            echo "Coverage data exported to $coverage_dir" > "$coverage_dir/coverage.txt"
        fi
    else
        warn "$component Swift unit tests failed"
        echo "FAILED" > "$component_dir/status.txt"
        
        if [ "$FAIL_FAST" = true ]; then
            error "$component Swift unit tests failed"
        fi
        return 1
    fi
    
    return 0
}

# Run unit tests for a component
run_component_unit_tests() {
    local component=$1
    
    case $component in
        "ring-buffer")
            run_ring_buffer_unit_tests
            ;;
        "packer"|"cli"|"benchmarks")
            run_rust_unit_tests "$component"
            ;;
        "collectors"|"ui")
            run_swift_unit_tests "$component"
            ;;
        *)
            error "Unknown component: $component"
            ;;
    esac
}

# Run all unit tests
run_all_unit_tests() {
    log "Running unit tests for components: ${COMPONENTS[*]}"
    
    if [ "$PARALLEL" = true ]; then
        # Run components in parallel
        local pids=()
        
        for component in "${COMPONENTS[@]}"; do
            (
                run_component_unit_tests "$component"
            ) &
            pids+=($!)
        done
        
        # Wait for all to complete
        local failed=false
        for pid in "${pids[@]}"; do
            if ! wait "$pid"; then
                failed=true
            fi
        done
        
        if [ "$failed" = true ]; then
            warn "Some unit tests failed"
            return 1
        fi
    else
        # Run components serially
        local failed=false
        
        for component in "${COMPONENTS[@]}"; do
            if ! run_component_unit_tests "$component"; then
                failed=true
                if [ "$FAIL_FAST" = true ]; then
                    error "Unit tests failed for $component"
                fi
            fi
        done
        
        if [ "$failed" = true ]; then
            warn "Some unit tests failed"
            return 1
        fi
    fi
    
    return 0
}

# Generate unit test summary
generate_unit_test_summary() {
    log "Generating unit test summary..."
    
    local summary_file="$OUTPUT_DIR/summary.txt"
    
    cat > "$summary_file" << EOF
Chronicle Unit Test Summary
Generated: $(date)

Configuration:
  Components: ${COMPONENTS[*]}
  Parallel: $PARALLEL
  Coverage: $COVERAGE
  Filter: ${FILTER:-none}

Results:
EOF
    
    local total_passed=0
    local total_failed=0
    
    for component in "${COMPONENTS[@]}"; do
        local component_dir="$OUTPUT_DIR/$component"
        
        if [ -f "$component_dir/status.txt" ]; then
            local status=$(cat "$component_dir/status.txt")
            echo "  $component: $status" >> "$summary_file"
            
            if [ "$status" = "PASSED" ]; then
                ((total_passed++))
            else
                ((total_failed++))
            fi
            
            # Add test statistics if available
            if [ -f "$component_dir/stats.txt" ]; then
                local stats=$(cat "$component_dir/stats.txt")
                echo "    Stats: $stats" >> "$summary_file"
            fi
        else
            echo "  $component: NOT RUN" >> "$summary_file"
        fi
    done
    
    echo "" >> "$summary_file"
    echo "Total: $total_passed passed, $total_failed failed" >> "$summary_file"
    
    # Add coverage summary if available
    if [ "$COVERAGE" = true ]; then
        echo "" >> "$summary_file"
        echo "Coverage Reports:" >> "$summary_file"
        
        for component in "${COMPONENTS[@]}"; do
            local coverage_dir="$OUTPUT_DIR/$component/coverage"
            if [ -d "$coverage_dir" ]; then
                echo "  $component: $coverage_dir/tarpaulin-report.html" >> "$summary_file"
            fi
        done
    fi
    
    log "Unit test summary saved to $summary_file"
}

# Main unit test function
main() {
    log "Starting Chronicle unit tests..."
    
    parse_args "$@"
    
    info "Unit test configuration:"
    info "  Components: ${COMPONENTS[*]}"
    info "  Parallel: $PARALLEL"
    info "  Coverage: $COVERAGE"
    info "  Fail Fast: $FAIL_FAST"
    info "  Filter: ${FILTER:-none}"
    info "  Output Directory: $OUTPUT_DIR"
    
    setup_unit_test_env
    
    if run_all_unit_tests; then
        generate_unit_test_summary
        log "Unit tests completed successfully!"
    else
        generate_unit_test_summary
        error "Some unit tests failed!"
    fi
}

# Run main function
main "$@"