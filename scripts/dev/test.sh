#!/bin/bash

# Chronicle Development Testing Script
# Runs tests for all Chronicle components

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
COMPONENTS=()
QUICK=false
PARALLEL=true
STOP_ON_FAILURE=false

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

Run tests for Chronicle components.

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -c, --coverage      Generate coverage reports
    -q, --quick         Run quick tests only (skip slow tests)
    -s, --serial        Run tests serially (not in parallel)
    -f, --fail-fast     Stop on first failure
    --unit              Run unit tests only
    --integration       Run integration tests only
    --performance       Run performance tests only
    --ring-buffer       Test only ring buffer component
    --packer            Test only packer component
    --collectors        Test only collectors component
    --cli               Test only CLI component
    --ui                Test only UI component
    --benchmarks        Test only benchmarks component

COMPONENTS:
    If no components are specified, all components will be tested.
    Available components: ring-buffer, packer, collectors, cli, ui, benchmarks, integration

EXAMPLES:
    $0                  # Run all tests
    $0 --quick          # Run quick tests only
    $0 --coverage cli   # Run CLI tests with coverage
    $0 --unit packer    # Run only packer unit tests

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
            -q|--quick)
                QUICK=true
                shift
                ;;
            -s|--serial)
                PARALLEL=false
                shift
                ;;
            -f|--fail-fast)
                STOP_ON_FAILURE=true
                shift
                ;;
            --unit)
                COMPONENTS+=("unit")
                shift
                ;;
            --integration)
                COMPONENTS+=("integration")
                shift
                ;;
            --performance)
                COMPONENTS+=("performance")
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
                COMPONENTS+=("$1")
                shift
                ;;
        esac
    done
    
    # If no components specified, test all
    if [ ${#COMPONENTS[@]} -eq 0 ]; then
        COMPONENTS=("ring-buffer" "packer" "collectors" "cli" "ui" "benchmarks" "integration")
    fi
}

# Test ring buffer component
test_ring_buffer() {
    log "Testing ring buffer component..."
    
    cd "$ROOT_DIR/ring-buffer"
    
    # Build test executable
    if ! make test; then
        error "Ring buffer test build failed"
    fi
    
    # Run tests
    if [ -f "./test_ring_buffer" ]; then
        if ! ./test_ring_buffer; then
            error "Ring buffer tests failed"
        fi
    else
        error "Ring buffer test executable not found"
    fi
    
    log "Ring buffer tests passed"
}

# Test Rust components
test_rust_component() {
    local component=$1
    local component_dir="$ROOT_DIR/$component"
    
    log "Testing Rust component: $component..."
    
    if [ ! -d "$component_dir" ]; then
        error "Component directory not found: $component_dir"
    fi
    
    cd "$component_dir"
    
    local cargo_flags=""
    if [ "$VERBOSE" = true ]; then
        cargo_flags="--verbose"
    fi
    
    if [ "$QUICK" = true ]; then
        cargo_flags="$cargo_flags --lib"
    fi
    
    # Run tests
    if [ "$COVERAGE" = true ]; then
        if ! cargo tarpaulin --out Html --output-dir coverage $cargo_flags; then
            error "$component tests failed"
        fi
        log "Coverage report generated in coverage/tarpaulin-report.html"
    else
        if ! cargo test $cargo_flags; then
            error "$component tests failed"
        fi
    fi
    
    # Run clippy for additional checks
    if ! cargo clippy -- -D warnings; then
        error "$component clippy checks failed"
    fi
    
    log "$component tests passed"
}

# Test Swift components
test_swift_component() {
    local component=$1
    local workspace_scheme=""
    
    log "Testing Swift component: $component..."
    
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
    
    if ! xcodebuild -workspace Chronicle.xcworkspace \
                    -scheme "$workspace_scheme" \
                    -configuration Debug \
                    -derivedDataPath build \
                    $xcode_flags \
                    test; then
        error "$component tests failed"
    fi
    
    log "$component tests passed"
}

# Run integration tests
test_integration() {
    log "Running integration tests..."
    
    cd "$ROOT_DIR/tests"
    
    local cargo_flags=""
    if [ "$VERBOSE" = true ]; then
        cargo_flags="--verbose"
    fi
    
    if [ "$QUICK" = true ]; then
        cargo_flags="$cargo_flags --test integration"
    fi
    
    if ! cargo test $cargo_flags; then
        error "Integration tests failed"
    fi
    
    log "Integration tests passed"
}

# Run performance tests
test_performance() {
    log "Running performance tests..."
    
    cd "$ROOT_DIR/benchmarks"
    
    local cargo_flags=""
    if [ "$VERBOSE" = true ]; then
        cargo_flags="--verbose"
    fi
    
    if ! cargo test $cargo_flags; then
        error "Performance tests failed"
    fi
    
    # Run benchmarks if not in quick mode
    if [ "$QUICK" = false ]; then
        log "Running benchmarks..."
        if ! cargo bench; then
            warn "Benchmarks failed but continuing..."
        fi
    fi
    
    log "Performance tests passed"
}

# Test a single component
test_component() {
    local component=$1
    
    case $component in
        "ring-buffer")
            test_ring_buffer
            ;;
        "packer")
            test_rust_component "packer"
            ;;
        "cli")
            test_rust_component "cli"
            ;;
        "benchmarks")
            test_performance
            ;;
        "collectors")
            test_swift_component "collectors"
            ;;
        "ui")
            test_swift_component "ui"
            ;;
        "integration")
            test_integration
            ;;
        "unit")
            # Run unit tests for all Rust components
            for rust_component in "packer" "cli"; do
                if [ -d "$ROOT_DIR/$rust_component" ]; then
                    test_rust_component "$rust_component"
                fi
            done
            ;;
        "performance")
            test_performance
            ;;
        *)
            error "Unknown component: $component"
            ;;
    esac
}

# Test all components
test_all() {
    log "Testing components: ${COMPONENTS[*]}"
    
    if [ "$PARALLEL" = true ]; then
        log "Running tests in parallel..."
        
        local pids=()
        for component in "${COMPONENTS[@]}"; do
            (
                test_component "$component"
            ) &
            pids+=($!)
        done
        
        # Wait for all tests to complete
        local failed=false
        for pid in "${pids[@]}"; do
            if ! wait "$pid"; then
                failed=true
                if [ "$STOP_ON_FAILURE" = true ]; then
                    error "Test failed, stopping due to --fail-fast"
                fi
            fi
        done
        
        if [ "$failed" = true ]; then
            error "One or more tests failed"
        fi
    else
        log "Running tests serially..."
        
        for component in "${COMPONENTS[@]}"; do
            test_component "$component"
            if [ $? -ne 0 ] && [ "$STOP_ON_FAILURE" = true ]; then
                error "Test failed, stopping due to --fail-fast"
            fi
        done
    fi
}

# Generate test summary
generate_summary() {
    log "Generating test summary..."
    
    cd "$ROOT_DIR"
    
    local summary_file="test_summary.txt"
    
    cat > "$summary_file" << EOF
Chronicle Test Summary
Generated: $(date)

Components tested: ${COMPONENTS[*]}
Configuration:
  - Quick mode: $QUICK
  - Coverage: $COVERAGE
  - Parallel: $PARALLEL
  - Verbose: $VERBOSE

Test Results:
EOF
    
    # Add individual component results
    for component in "${COMPONENTS[@]}"; do
        echo "  - $component: PASSED" >> "$summary_file"
    done
    
    if [ "$COVERAGE" = true ]; then
        echo "" >> "$summary_file"
        echo "Coverage reports generated in component-specific coverage/ directories" >> "$summary_file"
    fi
    
    log "Test summary saved to $summary_file"
}

# Main test function
main() {
    log "Starting Chronicle tests..."
    
    parse_args "$@"
    
    info "Test configuration:"
    info "  Components: ${COMPONENTS[*]}"
    info "  Quick mode: $QUICK"
    info "  Coverage: $COVERAGE"
    info "  Parallel: $PARALLEL"
    info "  Verbose: $VERBOSE"
    info "  Stop on failure: $STOP_ON_FAILURE"
    
    cd "$ROOT_DIR"
    
    test_all
    generate_summary
    
    log "All tests passed!"
    
    if [ "$COVERAGE" = true ]; then
        info "Coverage reports available in component-specific coverage/ directories"
    fi
}

# Run main function
main "$@"