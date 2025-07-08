#!/bin/bash

# Chronicle Docker Build Script
# Builds Chronicle components in Docker containers

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
VERBOSE=false
CLEAN=false
PLATFORM="linux/amd64"
TAG="chronicle:latest"
BUILD_CONTEXT="."
DOCKERFILE="Dockerfile"
PUSH=false
REGISTRY=""

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
ROOT_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"

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

Build Chronicle using Docker.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -c, --clean             Clean build (no cache)
    --platform PLATFORM    Target platform (default: linux/amd64)
    --tag TAG               Docker image tag (default: chronicle:latest)
    --dockerfile FILE       Dockerfile path (default: Dockerfile)
    --context DIR           Build context (default: .)
    --push                  Push to registry after build
    --registry REGISTRY     Registry to push to

EXAMPLES:
    $0                      # Basic build
    $0 --clean --push       # Clean build and push
    $0 --platform linux/arm64 --tag chronicle:arm64

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
            -c|--clean)
                CLEAN=true
                shift
                ;;
            --platform)
                PLATFORM="$2"
                shift 2
                ;;
            --tag)
                TAG="$2"
                shift 2
                ;;
            --dockerfile)
                DOCKERFILE="$2"
                shift 2
                ;;
            --context)
                BUILD_CONTEXT="$2"
                shift 2
                ;;
            --push)
                PUSH=true
                shift
                ;;
            --registry)
                REGISTRY="$2"
                shift 2
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

# Check Docker prerequisites
check_docker_prerequisites() {
    log "Checking Docker prerequisites..."
    
    # Check if Docker is available
    if ! command -v docker &> /dev/null; then
        error "Docker not found. Please install Docker."
    fi
    
    # Check if Docker daemon is running
    if ! docker info &> /dev/null; then
        error "Docker daemon is not running. Please start Docker."
    fi
    
    # Check if Dockerfile exists
    if [ ! -f "$BUILD_CONTEXT/$DOCKERFILE" ]; then
        warn "Dockerfile not found at $BUILD_CONTEXT/$DOCKERFILE"
        create_dockerfile
    fi
    
    log "Docker prerequisites check completed"
}

# Create Dockerfile if it doesn't exist
create_dockerfile() {
    log "Creating Dockerfile..."
    
    cat > "$BUILD_CONTEXT/Dockerfile" << 'EOF'
# Chronicle Docker Build
FROM rust:1.70 as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/chronicle

# Copy source code
COPY . .

# Build CLI tools
RUN cd cli && cargo build --release
RUN cd packer && cargo build --release

# Build ring buffer
RUN cd ring-buffer && make RELEASE=1

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries
COPY --from=builder /usr/src/chronicle/cli/target/release/chronicle /usr/local/bin/
COPY --from=builder /usr/src/chronicle/packer/target/release/chronicle-packer /usr/local/bin/
COPY --from=builder /usr/src/chronicle/ring-buffer/libringbuffer.a /usr/local/lib/

# Copy configuration
COPY --from=builder /usr/src/chronicle/config /usr/local/etc/chronicle/

# Create user
RUN useradd -r -s /bin/false chronicle

# Set permissions
RUN chmod +x /usr/local/bin/chronicle*

# Expose ports (if applicable)
# EXPOSE 8080

# Set default command
CMD ["chronicle", "--help"]
EOF
    
    info "Dockerfile created at $BUILD_CONTEXT/Dockerfile"
}

# Build Docker image
build_docker_image() {
    log "Building Docker image..."
    
    cd "$BUILD_CONTEXT"
    
    local docker_build_cmd="docker build"
    
    # Add platform if specified
    docker_build_cmd="$docker_build_cmd --platform $PLATFORM"
    
    # Add tag
    docker_build_cmd="$docker_build_cmd -t $TAG"
    
    # Add dockerfile
    docker_build_cmd="$docker_build_cmd -f $DOCKERFILE"
    
    # Add no-cache if clean build
    if [ "$CLEAN" = true ]; then
        docker_build_cmd="$docker_build_cmd --no-cache"
    fi
    
    # Add progress output if verbose
    if [ "$VERBOSE" = true ]; then
        docker_build_cmd="$docker_build_cmd --progress=plain"
    fi
    
    # Add build context
    docker_build_cmd="$docker_build_cmd ."
    
    # Execute build
    if ! eval "$docker_build_cmd"; then
        error "Docker build failed"
    fi
    
    log "Docker image built successfully: $TAG"
}

# Test Docker image
test_docker_image() {
    log "Testing Docker image..."
    
    # Test basic functionality
    if ! docker run --rm "$TAG" chronicle --version; then
        warn "Chronicle CLI test failed"
    else
        info "Chronicle CLI test passed"
    fi
    
    if ! docker run --rm "$TAG" chronicle-packer --help > /dev/null; then
        warn "Chronicle packer test failed"
    else
        info "Chronicle packer test passed"
    fi
    
    log "Docker image testing completed"
}

# Push Docker image
push_docker_image() {
    if [ "$PUSH" = false ]; then
        return 0
    fi
    
    log "Pushing Docker image..."
    
    local push_tag="$TAG"
    
    # Add registry prefix if specified
    if [ -n "$REGISTRY" ]; then
        push_tag="$REGISTRY/$TAG"
        
        # Tag for registry
        docker tag "$TAG" "$push_tag"
    fi
    
    # Push image
    if ! docker push "$push_tag"; then
        error "Failed to push Docker image: $push_tag"
    fi
    
    log "Docker image pushed successfully: $push_tag"
}

# Create multi-platform build
build_multiplatform() {
    log "Building multi-platform Docker image..."
    
    # Check if buildx is available
    if ! docker buildx version &> /dev/null; then
        error "Docker buildx not available. Please install buildx."
    fi
    
    # Create builder if it doesn't exist
    if ! docker buildx inspect multiplatform-builder &> /dev/null; then
        docker buildx create --name multiplatform-builder --use
    fi
    
    cd "$BUILD_CONTEXT"
    
    local buildx_cmd="docker buildx build"
    buildx_cmd="$buildx_cmd --platform linux/amd64,linux/arm64"
    buildx_cmd="$buildx_cmd -t $TAG"
    buildx_cmd="$buildx_cmd -f $DOCKERFILE"
    
    if [ "$PUSH" = true ]; then
        buildx_cmd="$buildx_cmd --push"
    else
        buildx_cmd="$buildx_cmd --load"
    fi
    
    if [ "$CLEAN" = true ]; then
        buildx_cmd="$buildx_cmd --no-cache"
    fi
    
    buildx_cmd="$buildx_cmd ."
    
    if ! eval "$buildx_cmd"; then
        error "Multi-platform Docker build failed"
    fi
    
    log "Multi-platform Docker build completed"
}

# Clean Docker artifacts
clean_docker_artifacts() {
    log "Cleaning Docker artifacts..."
    
    # Remove dangling images
    docker image prune -f
    
    # Remove build cache
    docker builder prune -f
    
    log "Docker cleanup completed"
}

# Generate Docker build report
generate_docker_report() {
    log "Generating Docker build report..."
    
    local report_file="docker-build-report.txt"
    
    cat > "$report_file" << EOF
Chronicle Docker Build Report
Generated: $(date)

Build Configuration:
  Platform: $PLATFORM
  Tag: $TAG
  Dockerfile: $DOCKERFILE
  Build Context: $BUILD_CONTEXT
  Clean Build: $CLEAN
  Push to Registry: $PUSH
  Registry: ${REGISTRY:-none}

Image Information:
EOF
    
    # Add image details
    if docker image inspect "$TAG" &> /dev/null; then
        echo "  Image ID: $(docker image inspect "$TAG" --format '{{.Id}}')" >> "$report_file"
        echo "  Size: $(docker image inspect "$TAG" --format '{{.Size}}' | numfmt --to=iec)" >> "$report_file"
        echo "  Created: $(docker image inspect "$TAG" --format '{{.Created}}')" >> "$report_file"
    fi
    
    log "Docker build report saved to: $report_file"
}

# Main Docker build function
main() {
    log "Starting Chronicle Docker build..."
    
    parse_args "$@"
    
    info "Docker build configuration:"
    info "  Platform: $PLATFORM"
    info "  Tag: $TAG"
    info "  Dockerfile: $DOCKERFILE"
    info "  Build Context: $BUILD_CONTEXT"
    info "  Clean: $CLEAN"
    info "  Push: $PUSH"
    info "  Registry: ${REGISTRY:-none}"
    
    check_docker_prerequisites
    
    # Check if multi-platform build is requested
    if [[ "$PLATFORM" =~ "," ]]; then
        build_multiplatform
    else
        build_docker_image
        test_docker_image
        push_docker_image
    fi
    
    generate_docker_report
    
    if [ "$CLEAN" = true ]; then
        clean_docker_artifacts
    fi
    
    log "Docker build completed successfully!"
    info "Image: $TAG"
    
    if [ "$PUSH" = true ] && [ -n "$REGISTRY" ]; then
        info "Pushed to: $REGISTRY/$TAG"
    fi
}

# Run main function
main "$@"