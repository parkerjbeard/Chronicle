# Chronicle Build System
.PHONY: all clean build-collectors build-packer build-cli build-ui test bench install dmg

# Configuration
XCODE_WORKSPACE = Chronicle.xcworkspace
CONFIGURATION = Release
DERIVED_DATA_PATH = build/DerivedData
PRODUCT_PATH = build/Products

# Rust configuration
CARGO_FLAGS = --release
RUST_LOG = info

all: build-collectors build-packer build-cli build-ui

clean:
	rm -rf build/
	cargo clean
	xcodebuild clean -workspace $(XCODE_WORKSPACE) -scheme ChronicleCollectors
	xcodebuild clean -workspace $(XCODE_WORKSPACE) -scheme ChronicleUI

# Build C ring buffer library
build-ring-buffer:
	@echo "Building ring buffer library..."
	cd ring-buffer && make clean && make

# Build Swift collectors
build-collectors: build-ring-buffer
	@echo "Building Swift collectors..."
	xcodebuild -workspace $(XCODE_WORKSPACE) \
		-scheme ChronicleCollectors \
		-configuration $(CONFIGURATION) \
		-derivedDataPath $(DERIVED_DATA_PATH) \
		build

# Build Rust packer service
build-packer:
	@echo "Building Rust packer service..."
	cd packer && cargo build $(CARGO_FLAGS)

# Build Rust CLI tool
build-cli:
	@echo "Building Rust CLI tool..."
	cd cli && cargo build $(CARGO_FLAGS)

# Build SwiftUI menu bar app
build-ui:
	@echo "Building SwiftUI menu bar app..."
	xcodebuild -workspace $(XCODE_WORKSPACE) \
		-scheme ChronicleUI \
		-configuration $(CONFIGURATION) \
		-derivedDataPath $(DERIVED_DATA_PATH) \
		build

# Run tests
test:
	@echo "Running C tests..."
	cd ring-buffer && make test
	@echo "Running Rust tests..."
	cd packer && cargo test
	cd cli && cargo test
	@echo "Running Swift tests..."
	xcodebuild test -workspace $(XCODE_WORKSPACE) \
		-scheme ChronicleCollectors \
		-configuration Debug \
		-derivedDataPath $(DERIVED_DATA_PATH)

# Run benchmarks
bench:
	@echo "Running benchmarks..."
	cd ring-buffer && make bench
	cd packer && cargo bench
	cd cli && cargo bench

# Install locally
install: all
	@echo "Installing Chronicle..."
	./scripts/install.sh

# Create DMG
dmg: all
	@echo "Creating DMG..."
	./scripts/create_dmg.sh

# Development helpers
dev-setup:
	@echo "Setting up development environment..."
	./scripts/dev_setup.sh

format:
	@echo "Formatting code..."
	cd packer && cargo fmt
	cd cli && cargo fmt
	swiftformat --config .swiftformat collectors/ ui/

lint:
	@echo "Linting code..."
	cd packer && cargo clippy -- -D warnings
	cd cli && cargo clippy -- -D warnings
	swiftlint --config .swiftlint.yml

docs:
	@echo "Building documentation..."
	cd packer && cargo doc --no-deps
	cd cli && cargo doc --no-deps