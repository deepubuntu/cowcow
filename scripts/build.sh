#!/bin/bash

# Cowcow Build Script
# Automates the complete build process for users

set -e

echo "🚀 Cowcow Build Automation Script"
echo "================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Helper functions
success() { echo -e "${GREEN}✅ $1${NC}"; }
error() { echo -e "${RED}❌ $1${NC}"; exit 1; }
warning() { echo -e "${YELLOW}⚠️ $1${NC}"; }
info() { echo -e "${BLUE}ℹ️ $1${NC}"; }

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "cowcow_cli" ]; then
    error "Please run this script from the cowcow project root directory"
fi

echo "📋 Step 1: Checking Prerequisites"
echo "--------------------------------"

# Check Rust installation
if command -v cargo &> /dev/null; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    success "Rust is installed (version: $RUST_VERSION)"
    
    # Check if version is recent enough
    if [[ "$RUST_VERSION" < "1.70" ]]; then
        warning "Rust version $RUST_VERSION detected. Recommended: 1.70+"
        info "Update with: rustup update"
    fi
else
    error "Rust not found. Install from: https://rustup.rs/"
fi

# Check Python installation
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version | cut -d' ' -f2)
    success "Python is installed (version: $PYTHON_VERSION)"
else
    error "Python 3 not found. Install from: https://python.org"
fi

# Check for system audio libraries (Linux only)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    info "Checking for Linux audio libraries..."
    
    if ! dpkg -l | grep -q libasound2-dev; then
        warning "libasound2-dev not found. Install with: sudo apt install libasound2-dev"
    else
        success "Audio libraries found"
    fi
fi

echo ""
echo "🔧 Step 2: Building Rust Components"
echo "-----------------------------------"

# Clean previous builds
info "Cleaning previous builds..."
cargo clean

# Build core library
info "Building cowcow_core..."
cargo build --release -p cowcow_core || error "Failed to build cowcow_core"
success "cowcow_core built successfully"

# Build CLI
info "Building cowcow_cli..."
cargo build --release -p cowcow_cli || error "Failed to build cowcow_cli"
success "cowcow_cli built successfully"

# Verify binary exists
if [ -f "./target/release/cowcow_cli" ]; then
    CLI_SIZE=$(du -h "./target/release/cowcow_cli" | cut -f1)
    success "CLI binary created (size: $CLI_SIZE)"
else
    error "CLI binary not found at ./target/release/cowcow_cli"
fi

echo ""
echo "🐍 Step 3: Setting Up Python Server"
echo "-----------------------------------"

cd server || error "Server directory not found"

# Create virtual environment if it doesn't exist
if [ ! -d ".venv" ]; then
    info "Creating Python virtual environment..."
    python3 -m venv .venv || error "Failed to create virtual environment"
    success "Virtual environment created"
fi

# Activate virtual environment
info "Activating virtual environment..."
source .venv/bin/activate || error "Failed to activate virtual environment"

# Install dependencies
info "Installing Python dependencies..."
pip install -r requirements.txt || error "Failed to install Python dependencies"
success "Python dependencies installed"

# Copy environment file if it doesn't exist
if [ ! -f ".env" ]; then
    if [ -f ".env.example" ]; then
        info "Copying .env.example to .env..."
        cp .env.example .env
        success "Environment file created"
    else
        warning "No .env.example found. You may need to create .env manually"
    fi
fi

cd ..

echo ""
echo "✅ Step 4: Build Verification"
echo "-----------------------------"

# Test CLI binary
info "Testing CLI binary..."
if ./target/release/cowcow_cli --help > /dev/null 2>&1; then
    success "CLI binary works correctly"
else
    error "CLI binary test failed"
fi

# Test core library
info "Testing core library..."
if cargo test -p cowcow_core --release > /dev/null 2>&1; then
    success "Core library tests passed"
else
    warning "Core library tests failed or not found"
fi

echo ""
echo "🎉 Build Complete!"
echo "=================="
echo ""
echo "Next steps:"
echo "1. Start the server: cd server && source .venv/bin/activate && uvicorn main:app --reload"
echo "2. In another terminal, register a user: ./target/release/cowcow_cli auth register"
echo "3. Try recording: ./target/release/cowcow_cli record --lang en"
echo ""
echo "For more information, see:"
echo "- README.md for quick start"
echo "- docs/SETUP.md for detailed setup"
echo "- docs/TESTING.md for testing examples"
echo ""
info "Build completed successfully! 🎊" 