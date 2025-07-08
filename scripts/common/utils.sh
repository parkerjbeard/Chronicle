#!/bin/bash

# Chronicle Common Utilities
# Shared functions and utilities for Chronicle scripts

# Colors for output
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export NC='\033[0m' # No Color

# Common logging functions
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

# Check if command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Check if running on macOS
is_macos() {
    [[ "$OSTYPE" = "darwin"* ]]
}

# Check if running as root
is_root() {
    [ "$EUID" -eq 0 ]
}

# Get CPU count
get_cpu_count() {
    if is_macos; then
        sysctl -n hw.ncpu
    else
        nproc
    fi
}

# Get available memory in MB
get_memory_mb() {
    if is_macos; then
        echo $(($(sysctl -n hw.memsize) / 1024 / 1024))
    else
        awk '/MemTotal/ {print int($2/1024)}' /proc/meminfo
    fi
}

# Create directory if it doesn't exist
ensure_dir() {
    local dir="$1"
    if [ ! -d "$dir" ]; then
        mkdir -p "$dir"
    fi
}

# Safe remove function
safe_remove() {
    local path="$1"
    if [ -e "$path" ]; then
        rm -rf "$path"
    fi
}

# Get file size in human readable format
get_file_size() {
    local file="$1"
    if [ -f "$file" ]; then
        if is_macos; then
            stat -f%z "$file" | numfmt --to=iec
        else
            stat -c%s "$file" | numfmt --to=iec
        fi
    else
        echo "0B"
    fi
}

# Check disk space
check_disk_space() {
    local path="$1"
    local required_mb="$2"
    
    if is_macos; then
        local available_mb=$(df -m "$path" | tail -1 | awk '{print $4}')
    else
        local available_mb=$(df -m "$path" | tail -1 | awk '{print $4}')
    fi
    
    if [ "$available_mb" -lt "$required_mb" ]; then
        return 1
    fi
    return 0
}

# Download file with progress
download_file() {
    local url="$1"
    local output="$2"
    
    if command_exists curl; then
        curl -L -o "$output" "$url"
    elif command_exists wget; then
        wget -O "$output" "$url"
    else
        error "No download tool available (curl or wget required)"
    fi
}

# Extract archive
extract_archive() {
    local archive="$1"
    local dest_dir="$2"
    
    ensure_dir "$dest_dir"
    
    case "$archive" in
        *.tar.gz|*.tgz)
            tar -xzf "$archive" -C "$dest_dir"
            ;;
        *.tar.bz2|*.tbz2)
            tar -xjf "$archive" -C "$dest_dir"
            ;;
        *.zip)
            unzip -q "$archive" -d "$dest_dir"
            ;;
        *.dmg)
            if is_macos; then
                hdiutil attach "$archive" -mountpoint "$dest_dir"
            else
                error "DMG files can only be mounted on macOS"
            fi
            ;;
        *)
            error "Unsupported archive format: $archive"
            ;;
    esac
}

# Find Chronicle root directory
find_chronicle_root() {
    local current_dir="$(pwd)"
    
    # Look for characteristic files
    while [ "$current_dir" != "/" ]; do
        if [ -f "$current_dir/Cargo.toml" ] || [ -f "$current_dir/Chronicle.xcworkspace" ]; then
            echo "$current_dir"
            return 0
        fi
        current_dir="$(dirname "$current_dir")"
    done
    
    return 1
}

# Get Chronicle version from source
get_chronicle_version() {
    local root_dir="${1:-$(find_chronicle_root)}"
    
    if [ -f "$root_dir/cli/Cargo.toml" ]; then
        grep '^version = ' "$root_dir/cli/Cargo.toml" | head -1 | sed 's/.*"\([^"]*\)".*/\1/'
    elif [ -f "$root_dir/Cargo.toml" ]; then
        grep '^version = ' "$root_dir/Cargo.toml" | head -1 | sed 's/.*"\([^"]*\)".*/\1/'
    else
        echo "unknown"
    fi
}

# Check Chronicle dependencies
check_chronicle_dependencies() {
    local missing_deps=()
    
    # Check for Rust
    if ! command_exists rustc; then
        missing_deps+=("rust")
    fi
    
    # Check for Cargo
    if ! command_exists cargo; then
        missing_deps+=("cargo")
    fi
    
    # Check for Xcode (on macOS)
    if is_macos && ! command_exists xcodebuild; then
        missing_deps+=("xcode")
    fi
    
    # Check for make
    if ! command_exists make; then
        missing_deps+=("make")
    fi
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        warn "Missing dependencies: ${missing_deps[*]}"
        return 1
    fi
    
    return 0
}

# Wait for process with timeout
wait_for_process() {
    local pid="$1"
    local timeout="${2:-30}"
    local count=0
    
    while [ $count -lt $timeout ]; do
        if ! kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
        sleep 1
        ((count++))
    done
    
    return 1
}

# Cleanup function for traps
cleanup_temp_files() {
    if [ -n "${TEMP_FILES:-}" ]; then
        for temp_file in $TEMP_FILES; do
            safe_remove "$temp_file"
        done
    fi
}

# Create temporary file
create_temp_file() {
    local prefix="${1:-chronicle}"
    local temp_file
    
    if is_macos; then
        temp_file=$(mktemp "/tmp/${prefix}.XXXXXX")
    else
        temp_file=$(mktemp -t "${prefix}.XXXXXX")
    fi
    
    # Add to cleanup list
    TEMP_FILES="${TEMP_FILES:-} $temp_file"
    
    echo "$temp_file"
}

# Lock file functions
acquire_lock() {
    local lock_file="$1"
    local timeout="${2:-30}"
    local count=0
    
    while [ $count -lt $timeout ]; do
        if (set -C; echo $$ > "$lock_file") 2>/dev/null; then
            return 0
        fi
        sleep 1
        ((count++))
    done
    
    return 1
}

release_lock() {
    local lock_file="$1"
    safe_remove "$lock_file"
}

# Configuration file functions
read_config() {
    local config_file="$1"
    local key="$2"
    
    if [ -f "$config_file" ]; then
        grep "^$key=" "$config_file" | cut -d'=' -f2- | tr -d '"'
    fi
}

write_config() {
    local config_file="$1"
    local key="$2"
    local value="$3"
    
    ensure_dir "$(dirname "$config_file")"
    
    if [ -f "$config_file" ]; then
        # Update existing key or add new one
        if grep -q "^$key=" "$config_file"; then
            sed -i.bak "s/^$key=.*/$key=\"$value\"/" "$config_file"
            rm -f "${config_file}.bak"
        else
            echo "$key=\"$value\"" >> "$config_file"
        fi
    else
        echo "$key=\"$value\"" > "$config_file"
    fi
}

# Version comparison
version_compare() {
    local version1="$1"
    local version2="$2"
    
    if [ "$version1" = "$version2" ]; then
        return 0
    fi
    
    local IFS=.
    local i ver1=($version1) ver2=($version2)
    
    for ((i=${#ver1[@]}; i<${#ver2[@]}; i++)); do
        ver1[i]=0
    done
    
    for ((i=0; i<${#ver1[@]}; i++)); do
        if [[ -z ${ver2[i]} ]]; then
            ver2[i]=0
        fi
        if ((10#${ver1[i]} > 10#${ver2[i]})); then
            return 1
        fi
        if ((10#${ver1[i]} < 10#${ver2[i]})); then
            return 2
        fi
    done
    return 0
}

# Network functions
check_internet_connection() {
    if command_exists curl; then
        curl -s --connect-timeout 5 http://www.google.com > /dev/null
    elif command_exists wget; then
        wget -q --spider --timeout=5 http://www.google.com
    elif command_exists ping; then
        ping -c 1 -W 5000 8.8.8.8 > /dev/null 2>&1
    else
        return 1
    fi
}

# Progress bar
show_progress() {
    local current="$1"
    local total="$2"
    local width=50
    local percentage=$((current * 100 / total))
    local completed=$((current * width / total))
    local remaining=$((width - completed))
    
    printf "\r["
    printf "%*s" $completed | tr ' ' '='
    printf "%*s" $remaining | tr ' ' '-'
    printf "] %d%%" $percentage
}

# Spinner for long-running operations
show_spinner() {
    local pid=$!
    local delay=0.1
    local spinstr='|/-\'
    
    while [ "$(ps a | awk '{print $1}' | grep $pid)" ]; do
        local temp=${spinstr#?}
        printf " [%c]  " "$spinstr"
        local spinstr=$temp${spinstr%"$temp"}
        sleep $delay
        printf "\b\b\b\b\b\b"
    done
    printf "    \b\b\b\b"
}

# Retry function
retry() {
    local max_attempts="$1"
    local delay="$2"
    shift 2
    local cmd="$*"
    
    local attempt=1
    while [ $attempt -le $max_attempts ]; do
        if eval "$cmd"; then
            return 0
        fi
        
        if [ $attempt -lt $max_attempts ]; then
            warn "Command failed (attempt $attempt/$max_attempts), retrying in ${delay}s..."
            sleep "$delay"
        fi
        
        ((attempt++))
    done
    
    error "Command failed after $max_attempts attempts: $cmd"
    return 1
}

# Setup trap for cleanup
setup_cleanup_trap() {
    trap cleanup_temp_files EXIT INT TERM
}

# Main initialization
init_common_utils() {
    # Set up cleanup trap
    setup_cleanup_trap
    
    # Set strict error handling
    set -euo pipefail
}

# Export functions for use in other scripts
export -f log warn error info
export -f command_exists is_macos is_root
export -f get_cpu_count get_memory_mb
export -f ensure_dir safe_remove get_file_size
export -f check_disk_space download_file extract_archive
export -f find_chronicle_root get_chronicle_version
export -f check_chronicle_dependencies
export -f wait_for_process cleanup_temp_files create_temp_file
export -f acquire_lock release_lock
export -f read_config write_config
export -f version_compare check_internet_connection
export -f show_progress show_spinner retry
export -f setup_cleanup_trap init_common_utils