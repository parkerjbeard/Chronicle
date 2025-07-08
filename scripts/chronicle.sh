#!/bin/bash

# Chronicle Main Script
# Entry point for all Chronicle build, test, and package operations

set -euo pipefail

# Script directory and root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Source common utilities
source "$SCRIPT_DIR/common/utils.sh"
source "$SCRIPT_DIR/common/config.sh"

# Initialize common utilities
init_common_utils
init_config

# Configuration
COMMAND=""
SUBCOMMAND=""
VERBOSE=false
DRY_RUN=false

# Show main usage
usage() {
    cat << EOF
Usage: $0 COMMAND [SUBCOMMAND] [OPTIONS]

Chronicle build, test, and package management script.

COMMANDS:
    dev         Development operations
    build       Build operations
    test        Testing operations
    package     Package creation
    install     Installation operations
    ci          CI/CD operations
    config      Configuration management
    help        Show help information

GLOBAL OPTIONS:
    -h, --help      Show this help message
    -v, --verbose   Enable verbose output
    -n, --dry-run   Show what would be done without executing
    --version       Show version information

EXAMPLES:
    $0 dev setup                    # Setup development environment
    $0 build release               # Build release version
    $0 test all                    # Run all tests
    $0 package dmg                 # Create DMG package
    $0 install local               # Install locally
    $0 ci github-actions build     # Run GitHub Actions build

For command-specific help:
    $0 COMMAND --help

EOF
}

# Show version information
show_version() {
    echo "Chronicle Build System"
    echo "Version: $CHRONICLE_VERSION"
    echo "Build Date: $CHRONICLE_BUILD_DATE"
    echo "Platform: $CHRONICLE_PLATFORM ($CHRONICLE_ARCH)"
    echo "Root Directory: $CHRONICLE_ROOT_DIR"
}

# Parse global arguments
parse_global_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -v|--verbose)
                VERBOSE=true
                export CHRONICLE_VERBOSE=true
                shift
                ;;
            -n|--dry-run)
                DRY_RUN=true
                export CHRONICLE_DRY_RUN=true
                shift
                ;;
            --version)
                show_version
                exit 0
                ;;
            -*)
                error "Unknown global option: $1"
                ;;
            *)
                # First non-option argument is the command
                if [ -z "$COMMAND" ]; then
                    COMMAND="$1"
                    shift
                    break
                fi
                ;;
        esac
    done
    
    # Remaining arguments are passed to subcommands
    export REMAINING_ARGS=("$@")
}

# Execute development commands
execute_dev_command() {
    local script_path="$SCRIPT_DIR/dev"
    
    case "${SUBCOMMAND:-}" in
        setup)
            exec "$script_path/setup.sh" "${REMAINING_ARGS[@]}"
            ;;
        build)
            exec "$script_path/build.sh" "${REMAINING_ARGS[@]}"
            ;;
        test)
            exec "$script_path/test.sh" "${REMAINING_ARGS[@]}"
            ;;
        clean)
            exec "$script_path/clean.sh" "${REMAINING_ARGS[@]}"
            ;;
        format)
            exec "$script_path/format.sh" "${REMAINING_ARGS[@]}"
            ;;
        ""|--help)
            cat << EOF
Usage: $0 dev SUBCOMMAND [OPTIONS]

Development operations for Chronicle.

SUBCOMMANDS:
    setup       Setup development environment
    build       Build in development mode
    test        Run development tests
    clean       Clean development artifacts
    format      Format source code

OPTIONS:
    -h, --help  Show this help message

EOF
            ;;
        *)
            error "Unknown dev subcommand: $SUBCOMMAND"
            ;;
    esac
}

# Execute build commands
execute_build_command() {
    local script_path="$SCRIPT_DIR/build"
    
    case "${SUBCOMMAND:-}" in
        release)
            exec "$script_path/build_release.sh" "${REMAINING_ARGS[@]}"
            ;;
        debug)
            exec "$script_path/build_debug.sh" "${REMAINING_ARGS[@]}"
            ;;
        universal)
            exec "$script_path/build_universal.sh" "${REMAINING_ARGS[@]}"
            ;;
        components)
            exec "$script_path/build_components.sh" "${REMAINING_ARGS[@]}"
            ;;
        sign)
            exec "$script_path/sign_and_notarize.sh" "${REMAINING_ARGS[@]}"
            ;;
        ""|--help)
            cat << EOF
Usage: $0 build SUBCOMMAND [OPTIONS]

Build operations for Chronicle.

SUBCOMMANDS:
    release     Build release version
    debug       Build debug version
    universal   Build universal binary
    components  Build individual components
    sign        Sign and notarize binaries

OPTIONS:
    -h, --help  Show this help message

EOF
            ;;
        *)
            error "Unknown build subcommand: $SUBCOMMAND"
            ;;
    esac
}

# Execute test commands
execute_test_command() {
    local script_path="$SCRIPT_DIR/test"
    
    case "${SUBCOMMAND:-}" in
        all)
            exec "$script_path/run_all_tests.sh" "${REMAINING_ARGS[@]}"
            ;;
        unit)
            exec "$script_path/run_unit_tests.sh" "${REMAINING_ARGS[@]}"
            ;;
        integration)
            exec "$script_path/run_integration_tests.sh" "${REMAINING_ARGS[@]}"
            ;;
        performance)
            exec "$script_path/run_performance_tests.sh" "${REMAINING_ARGS[@]}"
            ;;
        ci)
            exec "$script_path/run_ci_tests.sh" "${REMAINING_ARGS[@]}"
            ;;
        ""|--help)
            cat << EOF
Usage: $0 test SUBCOMMAND [OPTIONS]

Testing operations for Chronicle.

SUBCOMMANDS:
    all           Run all tests
    unit          Run unit tests
    integration   Run integration tests
    performance   Run performance tests
    ci            Run CI-optimized tests

OPTIONS:
    -h, --help    Show this help message

EOF
            ;;
        *)
            error "Unknown test subcommand: $SUBCOMMAND"
            ;;
    esac
}

# Execute package commands
execute_package_command() {
    local script_path="$SCRIPT_DIR/package"
    
    case "${SUBCOMMAND:-}" in
        dmg)
            exec "$script_path/create_dmg.sh" "${REMAINING_ARGS[@]}"
            ;;
        pkg)
            exec "$script_path/create_pkg.sh" "${REMAINING_ARGS[@]}"
            ;;
        zip)
            exec "$script_path/create_zip.sh" "${REMAINING_ARGS[@]}"
            ;;
        validate)
            exec "$script_path/validate_package.sh" "${REMAINING_ARGS[@]}"
            ;;
        upload)
            exec "$script_path/upload_release.sh" "${REMAINING_ARGS[@]}"
            ;;
        ""|--help)
            cat << EOF
Usage: $0 package SUBCOMMAND [OPTIONS]

Package creation operations for Chronicle.

SUBCOMMANDS:
    dmg         Create DMG package
    pkg         Create PKG installer
    zip         Create ZIP archive
    validate    Validate packages
    upload      Upload release packages

OPTIONS:
    -h, --help  Show this help message

EOF
            ;;
        *)
            error "Unknown package subcommand: $SUBCOMMAND"
            ;;
    esac
}

# Execute install commands
execute_install_command() {
    local script_path="$SCRIPT_DIR/install"
    
    case "${SUBCOMMAND:-}" in
        local)
            exec "$script_path/install_local.sh" "${REMAINING_ARGS[@]}"
            ;;
        system)
            exec "$script_path/install_system.sh" "${REMAINING_ARGS[@]}"
            ;;
        uninstall)
            exec "$script_path/uninstall.sh" "${REMAINING_ARGS[@]}"
            ;;
        update)
            exec "$script_path/update.sh" "${REMAINING_ARGS[@]}"
            ;;
        ""|--help)
            cat << EOF
Usage: $0 install SUBCOMMAND [OPTIONS]

Installation operations for Chronicle.

SUBCOMMANDS:
    local       Install locally for current user
    system      Install system-wide
    uninstall   Uninstall Chronicle
    update      Update Chronicle

OPTIONS:
    -h, --help  Show this help message

EOF
            ;;
        *)
            error "Unknown install subcommand: $SUBCOMMAND"
            ;;
    esac
}

# Execute CI commands
execute_ci_command() {
    local script_path="$SCRIPT_DIR/ci"
    
    case "${SUBCOMMAND:-}" in
        github-actions)
            exec "$script_path/github_actions.sh" "${REMAINING_ARGS[@]}"
            ;;
        gitlab-ci)
            exec "$script_path/gitlab_ci.sh" "${REMAINING_ARGS[@]}"
            ;;
        jenkins)
            exec "$script_path/jenkins.sh" "${REMAINING_ARGS[@]}"
            ;;
        docker)
            exec "$script_path/docker_build.sh" "${REMAINING_ARGS[@]}"
            ;;
        ""|--help)
            cat << EOF
Usage: $0 ci SUBCOMMAND [OPTIONS]

CI/CD operations for Chronicle.

SUBCOMMANDS:
    github-actions  GitHub Actions integration
    gitlab-ci       GitLab CI integration
    jenkins         Jenkins integration
    docker          Docker build

OPTIONS:
    -h, --help      Show this help message

EOF
            ;;
        *)
            error "Unknown ci subcommand: $SUBCOMMAND"
            ;;
    esac
}

# Execute config commands
execute_config_command() {
    case "${SUBCOMMAND:-}" in
        show)
            print_config
            ;;
        validate)
            if validate_config; then
                log "Configuration is valid"
            else
                error "Configuration validation failed"
            fi
            ;;
        save)
            local config_file="${REMAINING_ARGS[0]:-$CHRONICLE_ROOT_DIR/.chronicle/config.sh}"
            save_config "$config_file"
            ;;
        init)
            init_config
            log "Configuration initialized"
            ;;
        ""|--help)
            cat << EOF
Usage: $0 config SUBCOMMAND [OPTIONS]

Configuration management for Chronicle.

SUBCOMMANDS:
    show        Show current configuration
    validate    Validate configuration
    save FILE   Save current configuration to file
    init        Initialize configuration

OPTIONS:
    -h, --help  Show this help message

EOF
            ;;
        *)
            error "Unknown config subcommand: $SUBCOMMAND"
            ;;
    esac
}

# Main execution
main() {
    # Initialize arrays
    REMAINING_ARGS=()
    
    # Parse global arguments
    parse_global_args "$@"
    
    # If no command specified, show usage
    if [ -z "$COMMAND" ]; then
        usage
        exit 1
    fi
    
    # Extract subcommand if present
    if [ ${#REMAINING_ARGS[@]} -gt 0 ]; then
        SUBCOMMAND="${REMAINING_ARGS[0]}"
        REMAINING_ARGS=("${REMAINING_ARGS[@]:1}")
    else
        SUBCOMMAND=""
    fi
    
    # Execute command
    case "$COMMAND" in
        dev)
            execute_dev_command
            ;;
        build)
            execute_build_command
            ;;
        test)
            execute_test_command
            ;;
        package)
            execute_package_command
            ;;
        install)
            execute_install_command
            ;;
        ci)
            execute_ci_command
            ;;
        config)
            execute_config_command
            ;;
        help)
            usage
            ;;
        *)
            error "Unknown command: $COMMAND"
            ;;
    esac
}

# Run main function
main "$@"