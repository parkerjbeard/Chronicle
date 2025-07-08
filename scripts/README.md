# Chronicle Build Scripts

This directory contains the comprehensive build scripts and packaging system for Chronicle. The scripts are organized into logical categories and provide a complete automation solution for development, building, testing, packaging, and deployment.

## Quick Start

The main entry point is `chronicle.sh` which provides a unified interface to all build operations:

```bash
# Setup development environment
./scripts/chronicle.sh dev setup

# Build release version
./scripts/chronicle.sh build release

# Run all tests
./scripts/chronicle.sh test all

# Create DMG package
./scripts/chronicle.sh package dmg

# Install locally
./scripts/chronicle.sh install local
```

## Directory Structure

```
scripts/
├── chronicle.sh              # Main entry point script
├── common/
│   ├── utils.sh             # Shared utility functions
│   └── config.sh            # Configuration management
├── dev/                     # Development scripts
│   ├── setup.sh            # Development environment setup
│   ├── build.sh            # Development build
│   ├── test.sh             # Development testing
│   ├── clean.sh            # Clean development artifacts
│   └── format.sh           # Code formatting
├── build/                   # Build automation scripts
│   ├── build_release.sh    # Release builds
│   ├── build_debug.sh      # Debug builds
│   ├── build_universal.sh  # Universal binary builds
│   ├── build_components.sh # Component builds
│   └── sign_and_notarize.sh # Code signing and notarization
├── test/                    # Testing scripts
│   ├── run_all_tests.sh    # Run all tests
│   ├── run_unit_tests.sh   # Unit tests
│   ├── run_integration_tests.sh # Integration tests
│   ├── run_performance_tests.sh # Performance tests
│   └── run_ci_tests.sh     # CI-optimized tests
├── package/                 # Packaging scripts
│   ├── create_dmg.sh       # DMG creation
│   ├── create_pkg.sh       # PKG installer creation
│   ├── create_zip.sh       # ZIP archive creation
│   ├── validate_package.sh # Package validation
│   └── upload_release.sh   # Release upload
├── install/                 # Installation scripts
│   ├── install_local.sh    # Local installation
│   ├── install_system.sh   # System-wide installation
│   ├── uninstall.sh        # Uninstallation
│   └── update.sh           # Updates
└── ci/                      # CI/CD integration scripts
    ├── github_actions.sh   # GitHub Actions integration
    ├── gitlab_ci.sh        # GitLab CI integration
    ├── jenkins.sh          # Jenkins integration
    └── docker_build.sh     # Docker builds
```

## Command Categories

### Development Commands (`dev`)

Development scripts for setting up and working with Chronicle in development mode.

- `setup` - Sets up the development environment with all required dependencies
- `build` - Builds Chronicle in development mode with optimizations for development
- `test` - Runs development tests with fast feedback
- `clean` - Cleans development artifacts and build cache
- `format` - Formats source code using standard tools

### Build Commands (`build`)

Production build scripts for creating release versions of Chronicle.

- `release` - Creates optimized release builds with code signing and notarization
- `debug` - Creates debug builds with debugging symbols
- `universal` - Creates universal binaries for both Intel and Apple Silicon
- `components` - Builds individual components separately
- `sign` - Signs and notarizes existing binaries

### Test Commands (`test`)

Comprehensive testing suite with different test categories.

- `all` - Runs the complete test suite
- `unit` - Runs unit tests for individual components
- `integration` - Runs integration tests between components
- `performance` - Runs performance and benchmark tests
- `ci` - Runs CI-optimized tests with proper reporting

### Package Commands (`package`)

Creates distribution packages in various formats.

- `dmg` - Creates macOS DMG disk images with custom backgrounds
- `pkg` - Creates macOS PKG installers with proper metadata
- `zip` - Creates cross-platform ZIP archives
- `validate` - Validates created packages for correctness
- `upload` - Uploads packages to release repositories

### Install Commands (`install`)

Handles installation and updates of Chronicle.

- `local` - Installs Chronicle for the current user only
- `system` - Installs Chronicle system-wide (requires admin privileges)
- `uninstall` - Removes Chronicle from the system
- `update` - Updates Chronicle to the latest version

### CI Commands (`ci`)

CI/CD integration scripts for various platforms.

- `github-actions` - GitHub Actions integration with annotations and outputs
- `gitlab-ci` - GitLab CI integration with artifacts and caching
- `jenkins` - Jenkins integration with proper archiving
- `docker` - Docker-based builds for containerized environments

### Config Commands (`config`)

Configuration management utilities.

- `show` - Display current configuration
- `validate` - Validate configuration settings
- `save` - Save current configuration to file
- `init` - Initialize configuration with defaults

## Features

### Multi-Architecture Support
- Native builds for Intel (x86_64) and Apple Silicon (arm64)
- Universal binary creation for maximum compatibility
- Automatic architecture detection and optimization

### Code Signing and Notarization
- Automatic code signing with Developer ID certificates
- Notarization support for macOS Gatekeeper compliance
- Keychain integration for secure credential storage

### Error Handling and Logging
- Comprehensive error handling with meaningful messages
- Colored output for better readability
- Detailed logging with timestamps
- Progress indicators for long-running operations

### CI/CD Integration
- GitHub Actions integration with annotations and outputs
- GitLab CI support with artifacts and caching
- Jenkins integration with proper archiving
- Docker support for containerized builds

### Configuration Management
- Hierarchical configuration system
- Environment-specific settings
- Validation and error checking
- Easy customization and extension

## Usage Examples

### Basic Development Workflow

```bash
# Setup development environment
./scripts/chronicle.sh dev setup

# Build and test
./scripts/chronicle.sh dev build
./scripts/chronicle.sh dev test

# Format code
./scripts/chronicle.sh dev format

# Clean build artifacts
./scripts/chronicle.sh dev clean
```

### Release Building

```bash
# Create release build
./scripts/chronicle.sh build release --version 1.0.0

# Create universal binary
./scripts/chronicle.sh build universal

# Sign and notarize
./scripts/chronicle.sh build sign
```

### Testing

```bash
# Run all tests
./scripts/chronicle.sh test all

# Run specific test types
./scripts/chronicle.sh test unit
./scripts/chronicle.sh test integration
./scripts/chronicle.sh test performance
```

### Package Creation

```bash
# Create DMG package
./scripts/chronicle.sh package dmg --version 1.0.0

# Create PKG installer
./scripts/chronicle.sh package pkg --version 1.0.0

# Create ZIP archive
./scripts/chronicle.sh package zip --version 1.0.0 --type full
```

### CI/CD Usage

```bash
# GitHub Actions
./scripts/chronicle.sh ci github-actions setup
./scripts/chronicle.sh ci github-actions build
./scripts/chronicle.sh ci github-actions test

# Docker builds
./scripts/chronicle.sh ci docker --clean --push
```

## Configuration

The scripts use a hierarchical configuration system that loads settings from multiple sources:

1. System-wide: `/etc/chronicle/config.sh`
2. User-specific: `~/.chronicle/config.sh`
3. Project-specific: `Chronicle/.chronicle/config.sh`
4. Local overrides: `Chronicle/config/local.sh`
5. Environment variables

### Key Configuration Variables

- `CHRONICLE_VERSION` - Current version
- `CHRONICLE_RELEASE_MODE` - Enable release optimizations
- `CHRONICLE_PARALLEL_JOBS` - Number of parallel build jobs
- `CHRONICLE_CODE_SIGN` - Enable code signing
- `CHRONICLE_PACKAGE_FORMAT` - Package formats to create
- `CHRONICLE_TEST_TIMEOUT` - Test timeout in seconds

## Platform Support

### macOS
- Native support for macOS 10.14+
- Xcode integration for Swift components
- Code signing and notarization
- DMG and PKG package creation

### Linux
- Support for major Linux distributions
- Docker-based builds
- RPM and DEB package creation (planned)

## Error Handling

All scripts include comprehensive error handling:

- Strict error checking with `set -euo pipefail`
- Meaningful error messages with context
- Automatic cleanup of temporary files
- Lock file management to prevent concurrent runs
- Retry logic for network operations

## Development

### Adding New Scripts

1. Create the script in the appropriate category directory
2. Follow the existing naming conventions
3. Include comprehensive error handling
4. Add help text and usage examples
5. Update the main `chronicle.sh` script if needed

### Testing Scripts

Scripts can be tested individually or through the main entry point:

```bash
# Test individual script
./scripts/dev/build.sh --help

# Test through main entry point
./scripts/chronicle.sh dev build --help
```

### Debugging

Enable verbose output for debugging:

```bash
./scripts/chronicle.sh -v build release
```

Use dry-run mode to see what would be executed:

```bash
./scripts/chronicle.sh -n package dmg
```

## Contributing

When contributing to the build scripts:

1. Follow the existing code style and patterns
2. Include comprehensive error handling
3. Add help text and documentation
4. Test on multiple platforms if possible
5. Update this README as needed

## License

These scripts are part of the Chronicle project and follow the same license terms.