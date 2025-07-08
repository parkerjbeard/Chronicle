#!/bin/bash

# Chronicle Configuration Management
# Shared configuration settings and environment variables

# Version information
export CHRONICLE_VERSION="1.0.0"
export CHRONICLE_BUILD_DATE="$(date +%Y%m%d)"

# Directory structure
export CHRONICLE_ROOT_DIR="${CHRONICLE_ROOT_DIR:-$(dirname $(dirname $(dirname $(realpath ${BASH_SOURCE[0]}))))}"
export CHRONICLE_BUILD_DIR="${CHRONICLE_BUILD_DIR:-$CHRONICLE_ROOT_DIR/build}"
export CHRONICLE_DIST_DIR="${CHRONICLE_DIST_DIR:-$CHRONICLE_ROOT_DIR/dist}"
export CHRONICLE_SCRIPTS_DIR="${CHRONICLE_SCRIPTS_DIR:-$CHRONICLE_ROOT_DIR/scripts}"
export CHRONICLE_CONFIG_DIR="${CHRONICLE_CONFIG_DIR:-$CHRONICLE_ROOT_DIR/config}"
export CHRONICLE_LOGS_DIR="${CHRONICLE_LOGS_DIR:-$CHRONICLE_ROOT_DIR/logs}"

# Build configuration
export CHRONICLE_RELEASE_MODE="${CHRONICLE_RELEASE_MODE:-false}"
export CHRONICLE_VERBOSE="${CHRONICLE_VERBOSE:-false}"
export CHRONICLE_PARALLEL_JOBS="${CHRONICLE_PARALLEL_JOBS:-$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)}"
export CHRONICLE_CLEAN_BUILD="${CHRONICLE_CLEAN_BUILD:-false}"

# Rust configuration
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-auto}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
export RUSTC_WRAPPER="${RUSTC_WRAPPER:-}"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-1}"

# Build targets
export CHRONICLE_TARGETS="${CHRONICLE_TARGETS:-x86_64-apple-darwin aarch64-apple-darwin}"
export CHRONICLE_DEFAULT_TARGET="${CHRONICLE_DEFAULT_TARGET:-$(rustc -vV | grep 'host:' | cut -d' ' -f2)}"

# Code signing configuration
export CHRONICLE_CODE_SIGN="${CHRONICLE_CODE_SIGN:-auto}"
export CHRONICLE_DEVELOPER_ID="${CHRONICLE_DEVELOPER_ID:-}"
export CHRONICLE_KEYCHAIN_PROFILE="${CHRONICLE_KEYCHAIN_PROFILE:-}"
export CHRONICLE_NOTARIZATION_ENABLED="${CHRONICLE_NOTARIZATION_ENABLED:-auto}"

# Package configuration
export CHRONICLE_PACKAGE_FORMAT="${CHRONICLE_PACKAGE_FORMAT:-dmg,pkg,zip}"
export CHRONICLE_PACKAGE_PREFIX="${CHRONICLE_PACKAGE_PREFIX:-Chronicle}"
export CHRONICLE_BUNDLE_ID="${CHRONICLE_BUNDLE_ID:-com.chronicle.app}"

# Testing configuration
export CHRONICLE_TEST_TIMEOUT="${CHRONICLE_TEST_TIMEOUT:-300}"
export CHRONICLE_TEST_THREADS="${CHRONICLE_TEST_THREADS:-1}"
export CHRONICLE_TEST_OUTPUT_DIR="${CHRONICLE_TEST_OUTPUT_DIR:-test-results}"

# CI/CD configuration
export CHRONICLE_CI_MODE="${CHRONICLE_CI_MODE:-false}"
export CHRONICLE_ARTIFACT_RETENTION="${CHRONICLE_ARTIFACT_RETENTION:-30}"
export CHRONICLE_CACHE_ENABLED="${CHRONICLE_CACHE_ENABLED:-true}"

# Logging configuration
export CHRONICLE_LOG_LEVEL="${CHRONICLE_LOG_LEVEL:-info}"
export CHRONICLE_LOG_FILE="${CHRONICLE_LOG_FILE:-}"
export CHRONICLE_LOG_TIMESTAMP="${CHRONICLE_LOG_TIMESTAMP:-true}"

# Tool paths (auto-detected or specified)
export CHRONICLE_XCODE_PATH="${CHRONICLE_XCODE_PATH:-}"
export CHRONICLE_RUST_PATH="${CHRONICLE_RUST_PATH:-}"
export CHRONICLE_PYTHON_PATH="${CHRONICLE_PYTHON_PATH:-}"

# Platform-specific settings
case "$(uname -s)" in
    Darwin*)
        export CHRONICLE_PLATFORM="macos"
        export CHRONICLE_ARCH="$(uname -m)"
        export CHRONICLE_DEPLOYMENT_TARGET="${CHRONICLE_DEPLOYMENT_TARGET:-10.14}"
        export CHRONICLE_XCODE_REQUIRED="true"
        ;;
    Linux*)
        export CHRONICLE_PLATFORM="linux"
        export CHRONICLE_ARCH="$(uname -m)"
        export CHRONICLE_XCODE_REQUIRED="false"
        ;;
    *)
        export CHRONICLE_PLATFORM="unknown"
        export CHRONICLE_ARCH="unknown"
        export CHRONICLE_XCODE_REQUIRED="false"
        ;;
esac

# Feature flags
export CHRONICLE_FEATURE_RING_BUFFER="${CHRONICLE_FEATURE_RING_BUFFER:-true}"
export CHRONICLE_FEATURE_CLI="${CHRONICLE_FEATURE_CLI:-true}"
export CHRONICLE_FEATURE_PACKER="${CHRONICLE_FEATURE_PACKER:-true}"
export CHRONICLE_FEATURE_GUI="${CHRONICLE_FEATURE_GUI:-true}"

# Network settings
export CHRONICLE_NETWORK_TIMEOUT="${CHRONICLE_NETWORK_TIMEOUT:-30}"
export CHRONICLE_NETWORK_RETRIES="${CHRONICLE_NETWORK_RETRIES:-3}"

# Temporary directory
export CHRONICLE_TEMP_DIR="${CHRONICLE_TEMP_DIR:-${TMPDIR:-/tmp}/chronicle}"

# Lock file settings
export CHRONICLE_LOCK_DIR="${CHRONICLE_LOCK_DIR:-$CHRONICLE_TEMP_DIR/locks}"
export CHRONICLE_LOCK_TIMEOUT="${CHRONICLE_LOCK_TIMEOUT:-300}"

# Error handling
export CHRONICLE_STRICT_MODE="${CHRONICLE_STRICT_MODE:-true}"
export CHRONICLE_FAIL_FAST="${CHRONICLE_FAIL_FAST:-true}"

# Configuration validation
validate_config() {
    local errors=()
    
    # Check required directories exist
    if [ ! -d "$CHRONICLE_ROOT_DIR" ]; then
        errors+=("Chronicle root directory not found: $CHRONICLE_ROOT_DIR")
    fi
    
    # Check for required files
    if [ ! -f "$CHRONICLE_ROOT_DIR/Cargo.toml" ] && [ ! -f "$CHRONICLE_ROOT_DIR/Chronicle.xcworkspace" ]; then
        errors+=("Neither Cargo.toml nor Chronicle.xcworkspace found in root directory")
    fi
    
    # Validate build configuration
    if [ "$CHRONICLE_PARALLEL_JOBS" -lt 1 ] || [ "$CHRONICLE_PARALLEL_JOBS" -gt 32 ]; then
        errors+=("Invalid parallel jobs setting: $CHRONICLE_PARALLEL_JOBS (must be 1-32)")
    fi
    
    # Validate test timeout
    if [ "$CHRONICLE_TEST_TIMEOUT" -lt 10 ] || [ "$CHRONICLE_TEST_TIMEOUT" -gt 3600 ]; then
        errors+=("Invalid test timeout: $CHRONICLE_TEST_TIMEOUT (must be 10-3600 seconds)")
    fi
    
    # Check platform-specific requirements
    if [ "$CHRONICLE_PLATFORM" = "macos" ] && [ "$CHRONICLE_XCODE_REQUIRED" = "true" ]; then
        if ! command -v xcodebuild &> /dev/null; then
            errors+=("Xcode build tools required but not found")
        fi
    fi
    
    # Report errors
    if [ ${#errors[@]} -gt 0 ]; then
        echo "Configuration validation errors:" >&2
        for error in "${errors[@]}"; do
            echo "  - $error" >&2
        done
        return 1
    fi
    
    return 0
}

# Configuration loading
load_config_file() {
    local config_file="$1"
    
    if [ -f "$config_file" ]; then
        # Source the configuration file safely
        set -a  # Auto-export variables
        source "$config_file"
        set +a
        
        echo "Loaded configuration from: $config_file"
        return 0
    fi
    
    return 1
}

# Load configuration files in order of precedence
load_chronicle_config() {
    local config_loaded=false
    
    # 1. Load system-wide config
    if load_config_file "/etc/chronicle/config.sh"; then
        config_loaded=true
    fi
    
    # 2. Load user config
    if load_config_file "$HOME/.chronicle/config.sh"; then
        config_loaded=true
    fi
    
    # 3. Load project config
    if load_config_file "$CHRONICLE_ROOT_DIR/.chronicle/config.sh"; then
        config_loaded=true
    fi
    
    # 4. Load local config (not tracked by git)
    if load_config_file "$CHRONICLE_ROOT_DIR/config/local.sh"; then
        config_loaded=true
    fi
    
    # 5. Load environment-specific config
    if [ -n "${CHRONICLE_ENV:-}" ]; then
        if load_config_file "$CHRONICLE_ROOT_DIR/config/${CHRONICLE_ENV}.sh"; then
            config_loaded=true
        fi
    fi
    
    # Validate configuration after loading
    if ! validate_config; then
        echo "Configuration validation failed" >&2
        exit 1
    fi
    
    return 0
}

# Save current configuration
save_config() {
    local config_file="$1"
    local config_dir="$(dirname "$config_file")"
    
    # Create config directory if it doesn't exist
    mkdir -p "$config_dir"
    
    # Write current configuration
    cat > "$config_file" << EOF
#!/bin/bash
# Chronicle Configuration
# Generated: $(date)

# Build configuration
export CHRONICLE_RELEASE_MODE="$CHRONICLE_RELEASE_MODE"
export CHRONICLE_VERBOSE="$CHRONICLE_VERBOSE"
export CHRONICLE_PARALLEL_JOBS="$CHRONICLE_PARALLEL_JOBS"
export CHRONICLE_CLEAN_BUILD="$CHRONICLE_CLEAN_BUILD"

# Code signing
export CHRONICLE_CODE_SIGN="$CHRONICLE_CODE_SIGN"
export CHRONICLE_DEVELOPER_ID="$CHRONICLE_DEVELOPER_ID"
export CHRONICLE_KEYCHAIN_PROFILE="$CHRONICLE_KEYCHAIN_PROFILE"
export CHRONICLE_NOTARIZATION_ENABLED="$CHRONICLE_NOTARIZATION_ENABLED"

# Package configuration
export CHRONICLE_PACKAGE_FORMAT="$CHRONICLE_PACKAGE_FORMAT"
export CHRONICLE_PACKAGE_PREFIX="$CHRONICLE_PACKAGE_PREFIX"
export CHRONICLE_BUNDLE_ID="$CHRONICLE_BUNDLE_ID"

# Testing configuration
export CHRONICLE_TEST_TIMEOUT="$CHRONICLE_TEST_TIMEOUT"
export CHRONICLE_TEST_THREADS="$CHRONICLE_TEST_THREADS"
export CHRONICLE_TEST_OUTPUT_DIR="$CHRONICLE_TEST_OUTPUT_DIR"

# Feature flags
export CHRONICLE_FEATURE_RING_BUFFER="$CHRONICLE_FEATURE_RING_BUFFER"
export CHRONICLE_FEATURE_CLI="$CHRONICLE_FEATURE_CLI"
export CHRONICLE_FEATURE_PACKER="$CHRONICLE_FEATURE_PACKER"
export CHRONICLE_FEATURE_GUI="$CHRONICLE_FEATURE_GUI"
EOF
    
    echo "Configuration saved to: $config_file"
}

# Print current configuration
print_config() {
    echo "Chronicle Configuration:"
    echo "  Version: $CHRONICLE_VERSION"
    echo "  Platform: $CHRONICLE_PLATFORM ($CHRONICLE_ARCH)"
    echo "  Root Directory: $CHRONICLE_ROOT_DIR"
    echo "  Build Directory: $CHRONICLE_BUILD_DIR"
    echo "  Distribution Directory: $CHRONICLE_DIST_DIR"
    echo "  Release Mode: $CHRONICLE_RELEASE_MODE"
    echo "  Parallel Jobs: $CHRONICLE_PARALLEL_JOBS"
    echo "  Code Signing: $CHRONICLE_CODE_SIGN"
    echo "  Package Formats: $CHRONICLE_PACKAGE_FORMAT"
    echo "  Features: CLI=$CHRONICLE_FEATURE_CLI, Packer=$CHRONICLE_FEATURE_PACKER, GUI=$CHRONICLE_FEATURE_GUI"
}

# Initialize configuration
init_config() {
    # Ensure required directories exist
    mkdir -p "$CHRONICLE_BUILD_DIR"
    mkdir -p "$CHRONICLE_DIST_DIR"
    mkdir -p "$CHRONICLE_LOGS_DIR"
    mkdir -p "$CHRONICLE_TEMP_DIR"
    mkdir -p "$CHRONICLE_LOCK_DIR"
    
    # Set strict mode if enabled
    if [ "$CHRONICLE_STRICT_MODE" = "true" ]; then
        set -euo pipefail
    fi
    
    # Load configuration files
    load_chronicle_config
    
    echo "Chronicle configuration initialized"
}

# Export configuration functions
export -f validate_config load_config_file load_chronicle_config
export -f save_config print_config init_config