#!/bin/bash

# Cowcow Test Script
# Verifies that the system is working correctly

set -e

echo "🧪 Cowcow System Test"
echo "===================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PINK='\033[1;35m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0

# Beautiful loading bar function
loading_bar() {
    local message="$1"
    local duration=${2:-2}
    local width=40
    
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
        printf "]${NC} %d%%" $((progress * 100 / width))
        
        progress=$((progress + 1))
        sleep 0.05
    done
    echo ""
    echo ""
}

# Helper functions
success() { 
    echo -e "${GREEN}✅ $1${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

fail() { 
    echo -e "${RED}❌ $1${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

warning() { echo -e "${YELLOW}⚠️ $1${NC}"; }
info() { echo -e "${BLUE}ℹ️ $1${NC}"; }

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "cowcow_cli" ]; then
    fail "Please run this script from the cowcow project root directory"
    exit 1
fi

loading_bar "📋 Binary Verification..."

# Test CLI binary exists
if [ -f "./target/release/cowcow_cli" ]; then
    success "CLI binary exists"
else
    fail "CLI binary not found. Run: cargo build --release"
fi

# Test CLI binary works
if ./target/release/cowcow_cli --help > /dev/null 2>&1; then
    success "CLI binary responds to --help"
else
    fail "CLI binary help command failed"
fi

loading_bar "🏥 System Health Check..."

# Run built-in doctor command
if ./target/release/cowcow_cli doctor > /dev/null 2>&1; then
    success "System health check passed"
else
    fail "System health check failed"
fi

loading_bar "🔗 Server Connection Test..."

# Check if server is running
if curl -s -f http://localhost:8000/health > /dev/null 2>&1; then
    success "Server is running and healthy"
else
    fail "Server not running. Start with: cd server && uvicorn main:app --reload"
fi

loading_bar "🎛️ Configuration System..."

# Test config show command
if ./target/release/cowcow_cli config show > /dev/null 2>&1; then
    success "Configuration system works"
else
    fail "Configuration system failed"
fi

# Check if config file exists
if [ -f "$HOME/.cowcow/config.toml" ]; then
    success "Configuration file exists"
else
    warning "Configuration file not found (will be created on first run)"
fi

loading_bar "🔐 Authentication Test..."

# Test auth status
if ./target/release/cowcow_cli auth status > /dev/null 2>&1; then
    success "Authentication system responsive"
else
    fail "Authentication system failed"
fi

loading_bar "🎵 Audio System Check..."

# This is a basic test - we can't easily test microphone without user interaction
info "Audio system test requires manual verification"
info "Run: ./target/release/cowcow_cli record --lang test --duration 3"

echo ""
echo -e "${PINK}📊 Test Results${NC}"
echo -e "${PINK}===============${NC}"

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed! ($TESTS_PASSED/$TOTAL_TESTS)${NC}"
    echo ""
    echo -e "${PINK}╔═══════════════════════════════════════╗${NC}"
    echo -e "${PINK}║   Your Cowcow installation is working!   ║${NC}"
    echo -e "${PINK}╚═══════════════════════════════════════╝${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Register a user: ./target/release/cowcow_cli auth register"
    echo "2. Try recording: ./target/release/cowcow_cli record --lang en"
    echo "3. Upload recordings: ./target/release/cowcow_cli upload"
    exit 0
else
    echo -e "${RED}❌ Some tests failed ($TESTS_FAILED/$TOTAL_TESTS failed)${NC}"
    echo ""
    echo "Please check the failed tests above and:"
    echo "1. Ensure you've run: ./scripts/build.sh"
    echo "2. Start the server: cd server && uvicorn main:app --reload"
    echo "3. Check the setup guide: docs/SETUP.md"
    exit 1
fi 