#!/bin/bash

# Chronicle Development Environment Setup Script
# This script sets up the complete development environment for Chronicle

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
SCRIPTS_DIR="$(dirname "$SCRIPT_DIR")"

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

# Check if we're on macOS
check_macos() {
    if [[ "$OSTYPE" != "darwin"* ]]; then
        error "This script is designed for macOS only"
    fi
    log "Running on macOS"
}

# Check for required tools
check_dependencies() {
    log "Checking dependencies..."
    
    # Check for Xcode Command Line Tools
    if ! xcode-select -p &> /dev/null; then
        error "Xcode Command Line Tools not found. Please install with: xcode-select --install"
    fi
    
    # Check for Homebrew
    if ! command -v brew &> /dev/null; then
        warn "Homebrew not found. Installing..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi
    
    # Check for Rust
    if ! command -v rustc &> /dev/null; then
        warn "Rust not found. Installing..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    # Check for Python 3
    if ! command -v python3 &> /dev/null; then
        warn "Python 3 not found. Installing..."
        brew install python@3.11
    fi
    
    # Check for Node.js (for documentation generation)
    if ! command -v node &> /dev/null; then
        warn "Node.js not found. Installing..."
        brew install node
    fi
    
    log "Dependencies check complete"
}

# Install development tools
install_dev_tools() {
    log "Installing development tools..."
    
    # Rust tools
    if ! command -v cargo-audit &> /dev/null; then
        cargo install cargo-audit
    fi
    
    if ! command -v cargo-deny &> /dev/null; then
        cargo install cargo-deny
    fi
    
    if ! command -v cargo-outdated &> /dev/null; then
        cargo install cargo-outdated
    fi
    
    if ! command -v cargo-tarpaulin &> /dev/null; then
        cargo install cargo-tarpaulin
    fi
    
    # Install rustfmt and clippy
    rustup component add rustfmt
    rustup component add clippy
    
    # Install additional tools via Homebrew
    brew install --quiet jq
    brew install --quiet create-dmg
    brew install --quiet tree
    brew install --quiet wget
    brew install --quiet gh
    
    log "Development tools installed"
}

# Setup Python virtual environment
setup_python_env() {
    log "Setting up Python virtual environment..."
    
    cd "$ROOT_DIR"
    
    if [ ! -d "venv" ]; then
        python3 -m venv venv
    fi
    
    source venv/bin/activate
    
    # Install Python dependencies
    pip install --upgrade pip
    pip install pytest pytest-cov
    pip install black isort flake8
    pip install sphinx sphinx-rtd-theme
    
    log "Python environment setup complete"
}

# Setup project structure
setup_project_structure() {
    log "Setting up project structure..."
    
    cd "$ROOT_DIR"
    
    # Create necessary directories
    mkdir -p build/debug
    mkdir -p build/release
    mkdir -p build/universal
    mkdir -p build/artifacts
    mkdir -p dist/dmg
    mkdir -p dist/pkg
    mkdir -p dist/zip
    mkdir -p logs
    mkdir -p temp
    
    # Create config files if they don't exist
    if [ ! -f "config/chronicle.toml" ]; then
        if [ -f "config/chronicle.toml.example" ]; then
            cp config/chronicle.toml.example config/chronicle.toml
            log "Created config/chronicle.toml from example"
        fi
    fi
    
    log "Project structure setup complete"
}

# Setup git hooks
setup_git_hooks() {
    log "Setting up git hooks..."
    
    cd "$ROOT_DIR"
    
    # Create pre-commit hook
    cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
# Chronicle pre-commit hook

set -e

echo "Running pre-commit checks..."

# Run formatter
./scripts/dev/format.sh

# Run basic tests
./scripts/dev/test.sh --quick

echo "Pre-commit checks passed"
EOF
    
    chmod +x .git/hooks/pre-commit
    
    log "Git hooks setup complete"
}

# Verify installation
verify_installation() {
    log "Verifying installation..."
    
    cd "$ROOT_DIR"
    
    # Test Rust build
    info "Testing Rust build..."
    if ! cargo check --all; then
        error "Rust build check failed"
    fi
    
    # Test Swift build
    info "Testing Swift build..."
    if ! xcodebuild -workspace Chronicle.xcworkspace -scheme ChronicleCollectors -configuration Debug -quiet build; then
        error "Swift build check failed"
    fi
    
    # Test Python environment
    info "Testing Python environment..."
    if [ -f "venv/bin/activate" ]; then
        source venv/bin/activate
        if ! python -c "import pytest; print('Python environment OK')"; then
            error "Python environment check failed"
        fi
    fi
    
    log "Installation verification complete"
}

# Main setup function
main() {
    log "Starting Chronicle development environment setup..."
    
    check_macos
    check_dependencies
    install_dev_tools
    setup_python_env
    setup_project_structure
    setup_git_hooks
    verify_installation
    
    log "Development environment setup complete!"
    info "Next steps:"
    info "1. Run './scripts/dev/build.sh' to build the project"
    info "2. Run './scripts/dev/test.sh' to run tests"
    info "3. Use './scripts/dev/format.sh' to format code"
    info "4. Check './scripts/dev/clean.sh' to clean build artifacts"
}

# Run main function
main "$@"