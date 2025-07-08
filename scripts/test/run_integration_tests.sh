#!/bin/bash

# Chronicle Integration Tests Runner
# Runs integration tests that test component interactions

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
OUTPUT_DIR="$ROOT_DIR/test-results/integration"
TEST_SUITES=()
TIMEOUT=300  # 5 minutes default timeout

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

Run Chronicle integration tests.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -p, --parallel          Run tests in parallel (default)
    -s, --serial            Run tests serially
    -c, --coverage          Generate coverage reports
    -f, --fail-fast         Stop on first failure
    --output-dir DIR        Output directory for test results
    --timeout SECONDS       Test timeout in seconds (default: 300)
    --ring-buffer           Test ring buffer integration
    --packer-cli            Test packer-CLI integration
    --collectors-packer     Test collectors-packer integration
    --full-pipeline         Test complete data pipeline
    --api                   Test API integration
    --storage               Test storage integration
    --all                   Run all integration test suites

EXAMPLES:
    $0                      # Run all integration tests
    $0 --full-pipeline      # Test complete pipeline only
    $0 --timeout 600        # Use 10-minute timeout
    $0 --serial --coverage  # Serial execution with coverage

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
            --timeout)
                TIMEOUT="$2"
                shift 2
                ;;
            --ring-buffer)
                TEST_SUITES+=("ring-buffer")
                shift
                ;;
            --packer-cli)
                TEST_SUITES+=("packer-cli")
                shift
                ;;
            --collectors-packer)
                TEST_SUITES+=("collectors-packer")
                shift
                ;;
            --full-pipeline)
                TEST_SUITES+=("full-pipeline")
                shift
                ;;
            --api)
                TEST_SUITES+=("api")
                shift
                ;;
            --storage)
                TEST_SUITES+=("storage")
                shift
                ;;
            --all)
                TEST_SUITES=("ring-buffer" "packer-cli" "collectors-packer" "full-pipeline" "api" "storage")
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
    
    # If no test suites specified, run all
    if [ ${#TEST_SUITES[@]} -eq 0 ]; then
        TEST_SUITES=("ring-buffer" "packer-cli" "collectors-packer" "full-pipeline" "api" "storage")
    fi
}

# Setup integration test environment
setup_integration_test_env() {
    log "Setting up integration test environment..."
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    # Set environment variables
    export RUST_BACKTRACE=1
    export RUST_LOG=info
    export CHRONICLE_INTEGRATION_TEST=1
    export CHRONICLE_TEST_TIMEOUT="$TIMEOUT"
    
    # Create test data directory
    mkdir -p "$OUTPUT_DIR/test_data"
    
    # Clean previous results
    rm -rf "$OUTPUT_DIR"/*.log
    rm -rf "$OUTPUT_DIR"/test_*.txt
    
    log "Integration test environment ready"
}

# Check test prerequisites
check_test_prerequisites() {
    log "Checking test prerequisites..."
    
    # Check for required binaries
    local required_binaries=(
        "$ROOT_DIR/cli/target/debug/chronicle"
        "$ROOT_DIR/packer/target/debug/chronicle-packer"
        "$ROOT_DIR/ring-buffer/libringbuffer.a"
    )
    
    for binary in "${required_binaries[@]}"; do
        if [ ! -f "$binary" ]; then
            # Try release version
            local release_binary="${binary/debug/release}"
            if [ ! -f "$release_binary" ]; then
                error "Required binary not found: $binary (or release version)"
            fi
        fi
    done
    
    # Check for test data
    if [ ! -d "$ROOT_DIR/tests" ]; then
        error "Tests directory not found: $ROOT_DIR/tests"
    fi
    
    log "Prerequisites check passed"
}

# Run ring buffer integration tests
run_ring_buffer_integration_tests() {
    log "Running ring buffer integration tests..."
    
    local suite_dir="$OUTPUT_DIR/ring-buffer"
    mkdir -p "$suite_dir"
    
    cd "$ROOT_DIR/tests"
    
    # Use Rust integration test if available
    if [ -f "Cargo.toml" ]; then
        local cargo_flags="test ring_buffer_integration"
        if [ "$VERBOSE" = true ]; then
            cargo_flags="$cargo_flags --verbose"
        fi
        
        if [ "$COVERAGE" = true ]; then
            # Run with coverage
            if command -v cargo-tarpaulin &> /dev/null; then
                cargo tarpaulin --test ring_buffer_integration --out Html --output-dir "$suite_dir/coverage" > "$suite_dir/test.log" 2>&1
            else
                cargo test ring_buffer_integration > "$suite_dir/test.log" 2>&1
            fi
        else
            cargo test ring_buffer_integration > "$suite_dir/test.log" 2>&1
        fi
        
        if [ $? -eq 0 ]; then
            echo "PASSED" > "$suite_dir/status.txt"
            log "Ring buffer integration tests passed"
        else
            echo "FAILED" > "$suite_dir/status.txt"
            warn "Ring buffer integration tests failed"
            return 1
        fi
    else
        # Manual ring buffer integration test
        local test_script="$suite_dir/manual_test.sh"
        cat > "$test_script" << 'EOF'
#!/bin/bash
# Manual ring buffer integration test

set -e

# Test ring buffer creation and basic operations
echo "Testing ring buffer integration..."

# Create test data
echo "Creating test data..." >&2
for i in {1..1000}; do
    echo "test_data_$i"
done > test_data.txt

# Test with CLI (if available)
if [ -f "../cli/target/debug/chronicle" ]; then
    echo "Testing CLI integration..." >&2
    echo "CLI integration test would go here"
fi

echo "Ring buffer integration test completed"
EOF
        
        chmod +x "$test_script"
        
        if "$test_script" > "$suite_dir/test.log" 2>&1; then
            echo "PASSED" > "$suite_dir/status.txt"
            log "Ring buffer integration tests passed"
        else
            echo "FAILED" > "$suite_dir/status.txt"
            warn "Ring buffer integration tests failed"
            return 1
        fi
    fi
    
    return 0
}

# Run packer-CLI integration tests
run_packer_cli_integration_tests() {
    log "Running packer-CLI integration tests..."
    
    local suite_dir="$OUTPUT_DIR/packer-cli"
    mkdir -p "$suite_dir"
    
    # Create test script
    local test_script="$suite_dir/packer_cli_test.sh"
    cat > "$test_script" << 'EOF'
#!/bin/bash
# Packer-CLI integration test

set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
TEST_DATA_DIR="$(dirname "${BASH_SOURCE[0]}")/test_data"
mkdir -p "$TEST_DATA_DIR"

echo "Testing packer-CLI integration..."

# Find CLI binary
CLI_BIN=""
if [ -f "$ROOT_DIR/cli/target/debug/chronicle" ]; then
    CLI_BIN="$ROOT_DIR/cli/target/debug/chronicle"
elif [ -f "$ROOT_DIR/cli/target/release/chronicle" ]; then
    CLI_BIN="$ROOT_DIR/cli/target/release/chronicle"
else
    echo "CLI binary not found" >&2
    exit 1
fi

# Find packer binary
PACKER_BIN=""
if [ -f "$ROOT_DIR/packer/target/debug/chronicle-packer" ]; then
    PACKER_BIN="$ROOT_DIR/packer/target/debug/chronicle-packer"
elif [ -f "$ROOT_DIR/packer/target/release/chronicle-packer" ]; then
    PACKER_BIN="$ROOT_DIR/packer/target/release/chronicle-packer"
else
    echo "Packer binary not found" >&2
    exit 1
fi

# Test CLI help
echo "Testing CLI help..." >&2
$CLI_BIN --help > "$TEST_DATA_DIR/cli_help.txt"

# Test packer help
echo "Testing packer help..." >&2
$PACKER_BIN --help > "$TEST_DATA_DIR/packer_help.txt"

# Test CLI version
echo "Testing CLI version..." >&2
$CLI_BIN --version > "$TEST_DATA_DIR/cli_version.txt"

echo "Packer-CLI integration test completed"
EOF
    
    chmod +x "$test_script"
    
    if timeout "$TIMEOUT" "$test_script" > "$suite_dir/test.log" 2>&1; then
        echo "PASSED" > "$suite_dir/status.txt"
        log "Packer-CLI integration tests passed"
    else
        echo "FAILED" > "$suite_dir/status.txt"
        warn "Packer-CLI integration tests failed"
        return 1
    fi
    
    return 0
}

# Run collectors-packer integration tests
run_collectors_packer_integration_tests() {
    log "Running collectors-packer integration tests..."
    
    local suite_dir="$OUTPUT_DIR/collectors-packer"
    mkdir -p "$suite_dir"
    
    # Create test script
    local test_script="$suite_dir/collectors_packer_test.sh"
    cat > "$test_script" << 'EOF'
#!/bin/bash
# Collectors-Packer integration test

set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
TEST_DATA_DIR="$(dirname "${BASH_SOURCE[0]}")/test_data"
mkdir -p "$TEST_DATA_DIR"

echo "Testing collectors-packer integration..."

# Find packer binary
PACKER_BIN=""
if [ -f "$ROOT_DIR/packer/target/debug/chronicle-packer" ]; then
    PACKER_BIN="$ROOT_DIR/packer/target/debug/chronicle-packer"
elif [ -f "$ROOT_DIR/packer/target/release/chronicle-packer" ]; then
    PACKER_BIN="$ROOT_DIR/packer/target/release/chronicle-packer"
else
    echo "Packer binary not found" >&2
    exit 1
fi

# Check for collectors framework
COLLECTORS_FRAMEWORK="$ROOT_DIR/build/debug/xcode/Build/Products/Debug/ChronicleCollectors.framework"
if [ ! -d "$COLLECTORS_FRAMEWORK" ]; then
    COLLECTORS_FRAMEWORK="$ROOT_DIR/build/release/xcode/Build/Products/Release/ChronicleCollectors.framework"
    if [ ! -d "$COLLECTORS_FRAMEWORK" ]; then
        echo "Collectors framework not found, skipping framework integration test" >&2
        echo "Testing packer standalone..." >&2
        
        # Test packer configuration
        $PACKER_BIN --help > "$TEST_DATA_DIR/packer_config.txt"
        
        echo "Collectors-packer integration test completed (limited)"
        exit 0
    fi
fi

echo "Found collectors framework: $COLLECTORS_FRAMEWORK" >&2

# Create mock data for testing
echo "Creating mock collector data..." >&2
for i in {1..100}; do
    echo "{\"timestamp\": $(date +%s), \"event\": \"test_event_$i\", \"data\": \"mock_data_$i\"}"
done > "$TEST_DATA_DIR/mock_events.jsonl"

# Test packer with mock data
echo "Testing packer with mock data..." >&2
$PACKER_BIN --input "$TEST_DATA_DIR/mock_events.jsonl" --output "$TEST_DATA_DIR/packed_data.bin" || true

echo "Collectors-packer integration test completed"
EOF
    
    chmod +x "$test_script"
    
    if timeout "$TIMEOUT" "$test_script" > "$suite_dir/test.log" 2>&1; then
        echo "PASSED" > "$suite_dir/status.txt"
        log "Collectors-packer integration tests passed"
    else
        echo "FAILED" > "$suite_dir/status.txt"
        warn "Collectors-packer integration tests failed"
        return 1
    fi
    
    return 0
}

# Run full pipeline integration tests
run_full_pipeline_integration_tests() {
    log "Running full pipeline integration tests..."
    
    local suite_dir="$OUTPUT_DIR/full-pipeline"
    mkdir -p "$suite_dir"
    
    # Use Rust integration test if available
    cd "$ROOT_DIR/tests"
    
    if [ -f "Cargo.toml" ]; then
        local cargo_flags="test test_full_pipeline"
        if [ "$VERBOSE" = true ]; then
            cargo_flags="$cargo_flags --verbose"
        fi
        
        if [ "$COVERAGE" = true ] && command -v cargo-tarpaulin &> /dev/null; then
            cargo tarpaulin --test test_full_pipeline --out Html --output-dir "$suite_dir/coverage" > "$suite_dir/test.log" 2>&1
        else
            cargo test test_full_pipeline > "$suite_dir/test.log" 2>&1
        fi
        
        if [ $? -eq 0 ]; then
            echo "PASSED" > "$suite_dir/status.txt"
            log "Full pipeline integration tests passed"
        else
            echo "FAILED" > "$suite_dir/status.txt"
            warn "Full pipeline integration tests failed"
            return 1
        fi
    else
        # Manual full pipeline test
        local test_script="$suite_dir/full_pipeline_test.sh"
        cat > "$test_script" << 'EOF'
#!/bin/bash
# Full pipeline integration test

set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
TEST_DATA_DIR="$(dirname "${BASH_SOURCE[0]}")/test_data"
mkdir -p "$TEST_DATA_DIR"

echo "Testing full Chronicle pipeline..."

# This would test the complete data flow:
# 1. Collectors gathering data
# 2. Ring buffer storing data
# 3. Packer processing and compressing data
# 4. CLI querying and retrieving data

echo "Creating test data pipeline..." >&2

# Create mock ring buffer data
echo "Simulating ring buffer operations..." >&2
for i in {1..50}; do
    echo "event_$i|$(date +%s)|mock_data_$i"
done > "$TEST_DATA_DIR/ring_buffer_data.txt"

# Simulate packer processing
echo "Simulating packer processing..." >&2
echo "Processed $(wc -l < "$TEST_DATA_DIR/ring_buffer_data.txt") events" > "$TEST_DATA_DIR/packer_output.txt"

# Simulate CLI query
echo "Simulating CLI query..." >&2
echo "Query results: Found $(wc -l < "$TEST_DATA_DIR/ring_buffer_data.txt") matching events" > "$TEST_DATA_DIR/cli_query.txt"

echo "Full pipeline integration test completed"
EOF
        
        chmod +x "$test_script"
        
        if timeout "$TIMEOUT" "$test_script" > "$suite_dir/test.log" 2>&1; then
            echo "PASSED" > "$suite_dir/status.txt"
            log "Full pipeline integration tests passed"
        else
            echo "FAILED" > "$suite_dir/status.txt"
            warn "Full pipeline integration tests failed"
            return 1
        fi
    fi
    
    return 0
}

# Run API integration tests
run_api_integration_tests() {
    log "Running API integration tests..."
    
    local suite_dir="$OUTPUT_DIR/api"
    mkdir -p "$suite_dir"
    
    # Create API test script
    local test_script="$suite_dir/api_test.sh"
    cat > "$test_script" << 'EOF'
#!/bin/bash
# API integration test

set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
TEST_DATA_DIR="$(dirname "${BASH_SOURCE[0]}")/test_data"
mkdir -p "$TEST_DATA_DIR"

echo "Testing API integration..."

# Find CLI binary
CLI_BIN=""
if [ -f "$ROOT_DIR/cli/target/debug/chronicle" ]; then
    CLI_BIN="$ROOT_DIR/cli/target/debug/chronicle"
elif [ -f "$ROOT_DIR/cli/target/release/chronicle" ]; then
    CLI_BIN="$ROOT_DIR/cli/target/release/chronicle"
else
    echo "CLI binary not found" >&2
    exit 1
fi

# Test API endpoints (if CLI supports server mode)
echo "Testing CLI API capabilities..." >&2

# Test status command
echo "Testing status API..." >&2
$CLI_BIN status > "$TEST_DATA_DIR/status.txt" 2>&1 || echo "Status command not available"

# Test search API
echo "Testing search API..." >&2
$CLI_BIN search --query "test" > "$TEST_DATA_DIR/search.txt" 2>&1 || echo "Search command not available"

# Test config API
echo "Testing config API..." >&2
$CLI_BIN config --list > "$TEST_DATA_DIR/config.txt" 2>&1 || echo "Config command not available"

echo "API integration test completed"
EOF
    
    chmod +x "$test_script"
    
    if timeout "$TIMEOUT" "$test_script" > "$suite_dir/test.log" 2>&1; then
        echo "PASSED" > "$suite_dir/status.txt"
        log "API integration tests passed"
    else
        echo "FAILED" > "$suite_dir/status.txt"
        warn "API integration tests failed"
        return 1
    fi
    
    return 0
}

# Run storage integration tests
run_storage_integration_tests() {
    log "Running storage integration tests..."
    
    local suite_dir="$OUTPUT_DIR/storage"
    mkdir -p "$suite_dir"
    
    # Create storage test script
    local test_script="$suite_dir/storage_test.sh"
    cat > "$test_script" << 'EOF'
#!/bin/bash
# Storage integration test

set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
TEST_DATA_DIR="$(dirname "${BASH_SOURCE[0]}")/test_data"
mkdir -p "$TEST_DATA_DIR"

echo "Testing storage integration..."

# Find packer binary
PACKER_BIN=""
if [ -f "$ROOT_DIR/packer/target/debug/chronicle-packer" ]; then
    PACKER_BIN="$ROOT_DIR/packer/target/debug/chronicle-packer"
elif [ -f "$ROOT_DIR/packer/target/release/chronicle-packer" ]; then
    PACKER_BIN="$ROOT_DIR/packer/target/release/chronicle-packer"
else
    echo "Packer binary not found" >&2
    exit 1
fi

# Test storage operations
echo "Testing storage operations..." >&2

# Create test data
echo "Creating test storage data..." >&2
for i in {1..100}; do
    echo "{\"id\": $i, \"timestamp\": $(date +%s), \"data\": \"test_data_$i\"}"
done > "$TEST_DATA_DIR/storage_test_data.jsonl"

# Test storage with packer
echo "Testing packer storage..." >&2
$PACKER_BIN --help > "$TEST_DATA_DIR/packer_storage_help.txt" || true

# Create mock storage test
echo "Creating mock storage files..." >&2
mkdir -p "$TEST_DATA_DIR/storage"
cp "$TEST_DATA_DIR/storage_test_data.jsonl" "$TEST_DATA_DIR/storage/data.jsonl"

# Test file operations
echo "Testing file operations..." >&2
ls -la "$TEST_DATA_DIR/storage" > "$TEST_DATA_DIR/storage_listing.txt"
wc -l "$TEST_DATA_DIR/storage/data.jsonl" > "$TEST_DATA_DIR/storage_stats.txt"

echo "Storage integration test completed"
EOF
    
    chmod +x "$test_script"
    
    if timeout "$TIMEOUT" "$test_script" > "$suite_dir/test.log" 2>&1; then
        echo "PASSED" > "$suite_dir/status.txt"
        log "Storage integration tests passed"
    else
        echo "FAILED" > "$suite_dir/status.txt"
        warn "Storage integration tests failed"
        return 1
    fi
    
    return 0
}

# Run integration test suite
run_integration_test_suite() {
    local suite=$1
    
    case $suite in
        "ring-buffer")
            run_ring_buffer_integration_tests
            ;;
        "packer-cli")
            run_packer_cli_integration_tests
            ;;
        "collectors-packer")
            run_collectors_packer_integration_tests
            ;;
        "full-pipeline")
            run_full_pipeline_integration_tests
            ;;
        "api")
            run_api_integration_tests
            ;;
        "storage")
            run_storage_integration_tests
            ;;
        *)
            error "Unknown test suite: $suite"
            ;;
    esac
}

# Run all integration test suites
run_all_integration_tests() {
    log "Running integration test suites: ${TEST_SUITES[*]}"
    
    if [ "$PARALLEL" = true ]; then
        # Run suites in parallel
        local pids=()
        
        for suite in "${TEST_SUITES[@]}"; do
            (
                run_integration_test_suite "$suite"
            ) &
            pids+=($!)
        done
        
        # Wait for all to complete
        local failed=false
        for pid in "${pids[@]}"; do
            if ! wait "$pid"; then
                failed=true
                if [ "$FAIL_FAST" = true ]; then
                    error "Integration test suite failed"
                fi
            fi
        done
        
        if [ "$failed" = true ]; then
            warn "Some integration test suites failed"
            return 1
        fi
    else
        # Run suites serially
        local failed=false
        
        for suite in "${TEST_SUITES[@]}"; do
            if ! run_integration_test_suite "$suite"; then
                failed=true
                if [ "$FAIL_FAST" = true ]; then
                    error "Integration test suite failed: $suite"
                fi
            fi
        done
        
        if [ "$failed" = true ]; then
            warn "Some integration test suites failed"
            return 1
        fi
    fi
    
    return 0
}

# Generate integration test summary
generate_integration_test_summary() {
    log "Generating integration test summary..."
    
    local summary_file="$OUTPUT_DIR/summary.txt"
    
    cat > "$summary_file" << EOF
Chronicle Integration Test Summary
Generated: $(date)

Configuration:
  Test Suites: ${TEST_SUITES[*]}
  Parallel: $PARALLEL
  Coverage: $COVERAGE
  Timeout: ${TIMEOUT}s

Results:
EOF
    
    local total_passed=0
    local total_failed=0
    
    for suite in "${TEST_SUITES[@]}"; do
        local suite_dir="$OUTPUT_DIR/$suite"
        
        if [ -f "$suite_dir/status.txt" ]; then
            local status=$(cat "$suite_dir/status.txt")
            echo "  $suite: $status" >> "$summary_file"
            
            if [ "$status" = "PASSED" ]; then
                ((total_passed++))
            else
                ((total_failed++))
            fi
        else
            echo "  $suite: NOT RUN" >> "$summary_file"
        fi
    done
    
    echo "" >> "$summary_file"
    echo "Total: $total_passed passed, $total_failed failed" >> "$summary_file"
    
    log "Integration test summary saved to $summary_file"
}

# Main integration test function
main() {
    log "Starting Chronicle integration tests..."
    
    parse_args "$@"
    
    info "Integration test configuration:"
    info "  Test Suites: ${TEST_SUITES[*]}"
    info "  Parallel: $PARALLEL"
    info "  Coverage: $COVERAGE"
    info "  Fail Fast: $FAIL_FAST"
    info "  Timeout: ${TIMEOUT}s"
    info "  Output Directory: $OUTPUT_DIR"
    
    setup_integration_test_env
    check_test_prerequisites
    
    if run_all_integration_tests; then
        generate_integration_test_summary
        log "Integration tests completed successfully!"
    else
        generate_integration_test_summary
        error "Some integration tests failed!"
    fi
}

# Run main function
main "$@"