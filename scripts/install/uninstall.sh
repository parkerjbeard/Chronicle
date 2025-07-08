#!/bin/bash

# Chronicle Uninstall Script
# Removes Chronicle from the system

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VERBOSE=false
FORCE_REMOVE=false
REMOVE_CONFIG=false
REMOVE_DATA=false
UNINSTALL_TYPE="auto"

# Installation paths
LOCAL_BIN_DIR="$HOME/.local/bin"
LOCAL_CONFIG_DIR="$HOME/.config/chronicle"
LOCAL_DATA_DIR="$HOME/.local/share/chronicle"
LOCAL_APP_DIR="$HOME/Applications"

SYSTEM_BIN_DIR="/usr/local/bin"
SYSTEM_LIB_DIR="/usr/local/lib"
SYSTEM_SHARE_DIR="/usr/local/share/chronicle"
SYSTEM_CONFIG_DIR="/usr/local/etc/chronicle"
APPLICATIONS_DIR="/Applications"
DAEMON_PLIST="/Library/LaunchDaemons/com.chronicle.daemon.plist"

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

Uninstall Chronicle from the system.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -f, --force             Force removal without confirmation
    --remove-config         Remove configuration files
    --remove-data           Remove user data
    --local                 Uninstall local installation only
    --system                Uninstall system installation only (requires sudo)
    --all                   Remove everything including config and data

EXAMPLES:
    $0                      # Interactive uninstall
    $0 --force              # Force uninstall without prompts
    $0 --local              # Remove local installation only
    sudo $0 --system        # Remove system installation only
    $0 --all --force        # Remove everything without prompts

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
            -f|--force)
                FORCE_REMOVE=true
                shift
                ;;
            --remove-config)
                REMOVE_CONFIG=true
                shift
                ;;
            --remove-data)
                REMOVE_DATA=true
                shift
                ;;
            --local)
                UNINSTALL_TYPE="local"
                shift
                ;;
            --system)
                UNINSTALL_TYPE="system"
                shift
                ;;
            --all)
                REMOVE_CONFIG=true
                REMOVE_DATA=true
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

# Detect installed components
detect_installations() {
    log "Detecting Chronicle installations..."
    
    local local_found=false
    local system_found=false
    
    # Check for local installation
    if [ -f "$LOCAL_BIN_DIR/chronicle" ] || [ -f "$LOCAL_BIN_DIR/chronicle-packer" ] || \
       [ -d "$LOCAL_APP_DIR/Chronicle.app" ] || [ -d "$LOCAL_CONFIG_DIR" ]; then
        local_found=true
        info "Local installation found"
    fi
    
    # Check for system installation
    if [ -f "$SYSTEM_BIN_DIR/chronicle" ] || [ -f "$SYSTEM_BIN_DIR/chronicle-packer" ] || \
       [ -d "$APPLICATIONS_DIR/Chronicle.app" ] || [ -d "$SYSTEM_CONFIG_DIR" ] || \
       [ -f "$DAEMON_PLIST" ]; then
        system_found=true
        info "System installation found"
    fi
    
    # Auto-detect uninstall type
    if [ "$UNINSTALL_TYPE" = "auto" ]; then
        if [ "$local_found" = true ] && [ "$system_found" = true ]; then
            warn "Both local and system installations found"
            if [ "$FORCE_REMOVE" = false ]; then
                echo
                echo "Choose uninstall type:"
                echo "1) Local installation only"
                echo "2) System installation only"
                echo "3) Both installations"
                echo
                read -p "Enter choice [1-3]: " choice
                
                case $choice in
                    1) UNINSTALL_TYPE="local" ;;
                    2) UNINSTALL_TYPE="system" ;;
                    3) UNINSTALL_TYPE="both" ;;
                    *) error "Invalid choice" ;;
                esac
            else
                UNINSTALL_TYPE="both"
            fi
        elif [ "$local_found" = true ]; then
            UNINSTALL_TYPE="local"
        elif [ "$system_found" = true ]; then
            UNINSTALL_TYPE="system"
        else
            warn "No Chronicle installation found"
            return 1
        fi
    fi
    
    info "Uninstall type: $UNINSTALL_TYPE"
    
    # Check for root privileges if system uninstall
    if [[ "$UNINSTALL_TYPE" =~ ^(system|both)$ ]] && [ "$EUID" -ne 0 ]; then
        error "System uninstall requires root privileges. Use: sudo $0"
    fi
    
    return 0
}

# Confirm uninstall
confirm_uninstall() {
    if [ "$FORCE_REMOVE" = true ]; then
        return 0
    fi
    
    echo
    warn "This will remove Chronicle from your system."
    warn "Uninstall type: $UNINSTALL_TYPE"
    
    if [ "$REMOVE_CONFIG" = true ]; then
        warn "Configuration files will be removed"
    fi
    
    if [ "$REMOVE_DATA" = true ]; then
        warn "User data will be removed"
    fi
    
    echo
    read -p "Are you sure you want to continue? [y/N] " -n 1 -r
    echo
    
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        info "Uninstall cancelled"
        exit 0
    fi
}

# Stop running processes
stop_chronicle_processes() {
    log "Stopping Chronicle processes..."
    
    # Stop daemon if running
    if [ -f "$DAEMON_PLIST" ]; then
        log "Stopping system daemon..."
        launchctl stop com.chronicle.daemon 2>/dev/null || true
        launchctl unload "$DAEMON_PLIST" 2>/dev/null || true
    fi
    
    # Kill any running Chronicle processes
    pkill -f "chronicle" 2>/dev/null || true
    pkill -f "ChronicleUI" 2>/dev/null || true
    
    # Wait a moment for processes to terminate
    sleep 2
    
    log "Chronicle processes stopped"
}

# Remove local installation
remove_local_installation() {
    log "Removing local Chronicle installation..."
    
    # Remove binaries
    if [ -f "$LOCAL_BIN_DIR/chronicle" ]; then
        rm -f "$LOCAL_BIN_DIR/chronicle"
        info "Removed: $LOCAL_BIN_DIR/chronicle"
    fi
    
    if [ -f "$LOCAL_BIN_DIR/chronicle-packer" ]; then
        rm -f "$LOCAL_BIN_DIR/chronicle-packer"
        info "Removed: $LOCAL_BIN_DIR/chronicle-packer"
    fi
    
    # Remove app
    if [ -d "$LOCAL_APP_DIR/Chronicle.app" ]; then
        rm -rf "$LOCAL_APP_DIR/Chronicle.app"
        info "Removed: $LOCAL_APP_DIR/Chronicle.app"
    fi
    
    # Remove config if requested
    if [ "$REMOVE_CONFIG" = true ] && [ -d "$LOCAL_CONFIG_DIR" ]; then
        rm -rf "$LOCAL_CONFIG_DIR"
        info "Removed: $LOCAL_CONFIG_DIR"
    fi
    
    # Remove data if requested
    if [ "$REMOVE_DATA" = true ] && [ -d "$LOCAL_DATA_DIR" ]; then
        rm -rf "$LOCAL_DATA_DIR"
        info "Removed: $LOCAL_DATA_DIR"
    fi
    
    # Clean up shell profiles
    clean_shell_profiles_local
    
    log "Local installation removal completed"
}

# Remove system installation
remove_system_installation() {
    log "Removing system Chronicle installation..."
    
    # Remove daemon
    if [ -f "$DAEMON_PLIST" ]; then
        rm -f "$DAEMON_PLIST"
        info "Removed: $DAEMON_PLIST"
    fi
    
    # Remove binaries
    if [ -f "$SYSTEM_BIN_DIR/chronicle" ]; then
        rm -f "$SYSTEM_BIN_DIR/chronicle"
        info "Removed: $SYSTEM_BIN_DIR/chronicle"
    fi
    
    if [ -f "$SYSTEM_BIN_DIR/chronicle-packer" ]; then
        rm -f "$SYSTEM_BIN_DIR/chronicle-packer"
        info "Removed: $SYSTEM_BIN_DIR/chronicle-packer"
    fi
    
    if [ -f "$SYSTEM_BIN_DIR/chronicle-uninstall" ]; then
        rm -f "$SYSTEM_BIN_DIR/chronicle-uninstall"
        info "Removed: $SYSTEM_BIN_DIR/chronicle-uninstall"
    fi
    
    # Remove libraries
    if [ -f "$SYSTEM_LIB_DIR/libringbuffer.a" ]; then
        rm -f "$SYSTEM_LIB_DIR/libringbuffer.a"
        info "Removed: $SYSTEM_LIB_DIR/libringbuffer.a"
    fi
    
    if [ -d "$SYSTEM_LIB_DIR/ChronicleCollectors.framework" ]; then
        rm -rf "$SYSTEM_LIB_DIR/ChronicleCollectors.framework"
        info "Removed: $SYSTEM_LIB_DIR/ChronicleCollectors.framework"
    fi
    
    # Remove app
    if [ -d "$APPLICATIONS_DIR/Chronicle.app" ]; then
        rm -rf "$APPLICATIONS_DIR/Chronicle.app"
        info "Removed: $APPLICATIONS_DIR/Chronicle.app"
    fi
    
    # Remove config if requested
    if [ "$REMOVE_CONFIG" = true ] && [ -d "$SYSTEM_CONFIG_DIR" ]; then
        rm -rf "$SYSTEM_CONFIG_DIR"
        info "Removed: $SYSTEM_CONFIG_DIR"
    fi
    
    # Remove data/docs if requested
    if [ "$REMOVE_DATA" = true ] && [ -d "$SYSTEM_SHARE_DIR" ]; then
        rm -rf "$SYSTEM_SHARE_DIR"
        info "Removed: $SYSTEM_SHARE_DIR"
    fi
    
    # Clean up system PATH
    clean_system_path
    
    log "System installation removal completed"
}

# Clean shell profiles (local)
clean_shell_profiles_local() {
    log "Cleaning local shell profiles..."
    
    local shell_profiles=("$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile")
    
    for profile in "${shell_profiles[@]}"; do
        if [ -f "$profile" ]; then
            # Remove Chronicle PATH entries
            if grep -q "$LOCAL_BIN_DIR" "$profile"; then
                # Create backup
                cp "$profile" "${profile}.bak"
                
                # Remove Chronicle-related lines
                grep -v "$LOCAL_BIN_DIR" "$profile" > "${profile}.tmp" || true
                sed -i '' '/# Chronicle CLI tools/d' "${profile}.tmp" || true
                
                mv "${profile}.tmp" "$profile"
                
                if [ "$VERBOSE" = true ]; then
                    info "Cleaned Chronicle entries from $profile"
                fi
            fi
        fi
    done
    
    log "Local shell profiles cleaned"
}

# Clean system PATH
clean_system_path() {
    log "Cleaning system PATH..."
    
    # Remove from /etc/paths.d
    if [ -f "/etc/paths.d/chronicle" ]; then
        rm -f "/etc/paths.d/chronicle"
        info "Removed: /etc/paths.d/chronicle"
    fi
    
    # Clean system shell profiles
    local system_profiles=("/etc/profile" "/etc/bashrc" "/etc/zshrc")
    
    for profile in "${system_profiles[@]}"; do
        if [ -f "$profile" ]; then
            # Remove Chronicle PATH entries
            if grep -q "$SYSTEM_BIN_DIR" "$profile"; then
                # Create backup
                cp "$profile" "${profile}.bak"
                
                # Remove Chronicle-related lines
                grep -v "$SYSTEM_BIN_DIR" "$profile" > "${profile}.tmp" || true
                sed -i '' '/# Chronicle CLI tools/d' "${profile}.tmp" || true
                
                mv "${profile}.tmp" "$profile"
                
                if [ "$VERBOSE" = true ]; then
                    info "Cleaned Chronicle entries from $profile"
                fi
            fi
        fi
    done
    
    log "System PATH cleaned"
}

# Remove log files
remove_log_files() {
    log "Removing log files..."
    
    # System logs
    if [ -f "/var/log/chronicle.log" ]; then
        rm -f "/var/log/chronicle.log"
        info "Removed: /var/log/chronicle.log"
    fi
    
    if [ -f "/var/log/chronicle.error.log" ]; then
        rm -f "/var/log/chronicle.error.log"
        info "Removed: /var/log/chronicle.error.log"
    fi
    
    # User logs
    local user_logs=("$HOME/.cache/chronicle" "$HOME/Library/Logs/Chronicle")
    
    for log_dir in "${user_logs[@]}"; do
        if [ -d "$log_dir" ]; then
            rm -rf "$log_dir"
            info "Removed: $log_dir"
        fi
    done
    
    log "Log files removal completed"
}

# Remove application support files
remove_app_support() {
    log "Removing application support files..."
    
    # macOS application support
    local app_support_dirs=(
        "$HOME/Library/Application Support/Chronicle"
        "$HOME/Library/Caches/com.chronicle"
        "$HOME/Library/Preferences/com.chronicle.plist"
    )
    
    for item in "${app_support_dirs[@]}"; do
        if [ -e "$item" ]; then
            rm -rf "$item"
            info "Removed: $item"
        fi
    done
    
    log "Application support files removal completed"
}

# Generate uninstall report
generate_uninstall_report() {
    log "Generating uninstall report..."
    
    local report_file="/tmp/chronicle_uninstall_report.txt"
    
    cat > "$report_file" << EOF
Chronicle Uninstall Report
Generated: $(date)

Uninstall Configuration:
  Type: $UNINSTALL_TYPE
  Remove Config: $REMOVE_CONFIG
  Remove Data: $REMOVE_DATA
  Force Remove: $FORCE_REMOVE

Removed Components:
EOF
    
    case $UNINSTALL_TYPE in
        "local"|"both")
            echo "  Local Installation:" >> "$report_file"
            echo "    - CLI tools from $LOCAL_BIN_DIR" >> "$report_file"
            echo "    - App from $LOCAL_APP_DIR" >> "$report_file"
            if [ "$REMOVE_CONFIG" = true ]; then
                echo "    - Configuration from $LOCAL_CONFIG_DIR" >> "$report_file"
            fi
            if [ "$REMOVE_DATA" = true ]; then
                echo "    - Data from $LOCAL_DATA_DIR" >> "$report_file"
            fi
            ;;
    esac
    
    case $UNINSTALL_TYPE in
        "system"|"both")
            echo "  System Installation:" >> "$report_file"
            echo "    - CLI tools from $SYSTEM_BIN_DIR" >> "$report_file"
            echo "    - Libraries from $SYSTEM_LIB_DIR" >> "$report_file"
            echo "    - App from $APPLICATIONS_DIR" >> "$report_file"
            echo "    - Launch daemon" >> "$report_file"
            if [ "$REMOVE_CONFIG" = true ]; then
                echo "    - Configuration from $SYSTEM_CONFIG_DIR" >> "$report_file"
            fi
            if [ "$REMOVE_DATA" = true ]; then
                echo "    - Data from $SYSTEM_SHARE_DIR" >> "$report_file"
            fi
            ;;
    esac
    
    echo "" >> "$report_file"
    echo "Cleanup Actions:" >> "$report_file"
    echo "  - Stopped running processes" >> "$report_file"
    echo "  - Cleaned shell profiles" >> "$report_file"
    echo "  - Removed log files" >> "$report_file"
    echo "  - Removed application support files" >> "$report_file"
    
    echo
    info "Uninstall report saved to: $report_file"
    
    if [ "$VERBOSE" = true ]; then
        echo
        cat "$report_file"
    fi
}

# Main uninstall function
main() {
    log "Starting Chronicle uninstall..."
    
    parse_args "$@"
    
    if ! detect_installations; then
        info "No Chronicle installation found"
        exit 0
    fi
    
    confirm_uninstall
    stop_chronicle_processes
    
    case $UNINSTALL_TYPE in
        "local")
            remove_local_installation
            ;;
        "system")
            remove_system_installation
            ;;
        "both")
            remove_local_installation
            remove_system_installation
            ;;
    esac
    
    remove_log_files
    remove_app_support
    generate_uninstall_report
    
    log "Chronicle uninstall completed successfully!"
    
    echo
    info "Chronicle has been removed from your system"
    
    if [ "$REMOVE_CONFIG" = false ]; then
        info "Configuration files were preserved"
    fi
    
    if [ "$REMOVE_DATA" = false ]; then
        info "User data was preserved"
    fi
    
    echo
    info "Please restart your terminal to update PATH"
}

# Run main function
main "$@"