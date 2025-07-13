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
PINK='\033[1;35m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Beautiful loading bar function
loading_bar() {
    local message="$1"
    local duration=${2:-3}
    local width=50
    
    echo -e "${PINK}$message${NC}"
    echo -e "${MAGENTA}....${NC}"
    echo -e "${PINK}....${NC}"
    echo -e "${RED}....${NC}"
    echo -e "${MAGENTA}....${NC}"
    echo ""
    
    local step=$((duration * 10))
    local progress=0
    
    while [ $progress -le $width ]; do
        printf "\r${PINK}["
        for ((i=0; i<progress; i++)); do
            printf "█"
        done
        for ((i=progress; i<width; i++)); do
            printf "."
        done
        printf "]${NC} %d%%" $((progress * 2))
        
        progress=$((progress + 1))
        sleep 0.06
    done
    echo ""
    echo ""
}

# Helper functions
success() { echo -e "${GREEN}✅ $1${NC}"; }
error() { echo -e "${RED}❌ $1${NC}"; exit 1; }
warning() { echo -e "${YELLOW}⚠️ $1${NC}"; }
info() { echo -e "${BLUE}ℹ️ $1${NC}"; }

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "cowcow_cli" ]; then
    error "Please run this script from the cowcow project root directory"
fi

loading_bar "📋 Checking Prerequisites..."

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

# Check Python installation - prefer 3.11 or 3.12 for better compatibility
if command -v python3.11 &> /dev/null; then
    PYTHON_CMD="python3.11"
    PYTHON_VERSION=$(python3.11 --version | cut -d' ' -f2)
    success "Python is installed (version: $PYTHON_VERSION)"
elif command -v python3.12 &> /dev/null; then
    PYTHON_CMD="python3.12"
    PYTHON_VERSION=$(python3.12 --version | cut -d' ' -f2)
    success "Python is installed (version: $PYTHON_VERSION)"
elif command -v python3 &> /dev/null; then
    PYTHON_CMD="python3"
    PYTHON_VERSION=$(python3 --version | cut -d' ' -f2)
    success "Python is installed (version: $PYTHON_VERSION)"
    if [[ "$PYTHON_VERSION" > "3.13" ]]; then
        warning "Python 3.13+ detected. Some packages may have compatibility issues."
    fi
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

loading_bar "🔧 Building Rust Components..." 4

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

loading_bar "🐍 Setting Up Python Server..." 3

cd server || error "Server directory not found"

# Create virtual environment if it doesn't exist
if [ ! -d ".venv" ]; then
    info "Creating Python virtual environment..."
    $PYTHON_CMD -m venv .venv || error "Failed to create virtual environment"
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

loading_bar "✅ Running Build Verification..." 2

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
echo -e "${PINK}🎉 Build Complete!${NC}"
echo -e "${PINK}==================${NC}"
echo ""
echo -e "${PINK}╔═══════════════════════════════════════╗${NC}"
echo -e "${PINK}║       🚀 Next Steps & Verification    ║${NC}"
echo -e "${PINK}╚═══════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}🧪 Step 1: Verify Your Build${NC}"
echo "   Run the test suite to ensure everything works:"
echo "   ${YELLOW}./scripts/test.sh${NC}"
echo ""
echo -e "${BLUE}🖥️  Step 2: Start the Server${NC}"
echo "   In a terminal window:"
echo "   ${YELLOW}cd server && source .venv/bin/activate${NC}"
echo "   ${YELLOW}uvicorn main:app --reload${NC}"
echo ""
echo -e "${BLUE}👤 Step 3: Register a User Account${NC}"
echo "   In another terminal:"
echo "   ${YELLOW}./target/release/cowcow_cli auth register${NC}"
echo ""
echo -e "${BLUE}🎙️  Step 4: Test Recording${NC}"
echo "   Try your first recording:"
echo "   ${YELLOW}./target/release/cowcow_cli record --lang en --duration 5${NC}"
echo ""
echo -e "${BLUE}📤 Step 5: Upload Your Recording${NC}"
echo "   Upload to the server:"
echo "   ${YELLOW}./target/release/cowcow_cli upload${NC}"
echo ""
echo -e "${BLUE}📊 Step 6: Check Your Stats${NC}"
echo "   View recording statistics:"
echo "   ${YELLOW}./target/release/cowcow_cli stats${NC}"
echo ""
echo -e "${GREEN}🏥 Health Check Commands:${NC}"
echo "• System health: ${YELLOW}./target/release/cowcow_cli doctor${NC}"
echo "• Auth status: ${YELLOW}./target/release/cowcow_cli auth status${NC}"
echo "• Config view: ${YELLOW}./target/release/cowcow_cli config show${NC}"
echo ""
echo -e "${BLUE}📚 Documentation:${NC}"
echo "• Quick start: ${YELLOW}README.md${NC}"
echo "• Detailed setup: ${YELLOW}docs/SETUP.md${NC}"
echo "• Testing guide: ${YELLOW}docs/TESTING.md${NC}"
echo ""
echo -e "${YELLOW}⚠️  Troubleshooting:${NC}"
echo "• If tests fail, check server is running and try: ${YELLOW}./scripts/build.sh${NC}"
echo "• For audio issues, ensure microphone permissions are granted"
echo "• For server issues, check ${YELLOW}server/.env${NC} configuration"
echo ""
echo -e "${PINK}╔═══════════════════════════════════════╗${NC}"
echo -e "${PINK}║        Built with ❤️ by Thabhelo        ${NC}"
echo -e "${PINK}╚═══════════════════════════════════════╝${NC}"
echo ""
info "Build completed successfully! Run './scripts/test.sh' to verify everything works! 🎊" 