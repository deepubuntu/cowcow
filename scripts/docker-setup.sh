#!/bin/bash
# CowCow Docker Setup Script
# This script helps you set up CowCow with Docker for development or production

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}[COWCOW]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to generate random string
generate_random() {
    openssl rand -hex 32
}

# Check dependencies
check_dependencies() {
    print_header "Checking dependencies..."
    
    if ! command_exists docker; then
        print_error "Docker is not installed. Please install Docker first."
        exit 1
    fi
    
    if ! command_exists docker-compose; then
        print_error "Docker Compose is not installed. Please install Docker Compose first."
        exit 1
    fi
    
    print_status "All dependencies are available."
}

# Setup environment file
setup_environment() {
    print_header "Setting up environment configuration..."
    
    if [ ! -f .env ]; then
        if [ -f env.example ]; then
            cp env.example .env
            print_status "Created .env file from env.example"
        else
            print_error "env.example file not found!"
            exit 1
        fi
    else
        print_warning ".env file already exists. Skipping environment setup."
        return
    fi
    
    # Generate JWT secret
    JWT_SECRET=$(generate_random)
    sed -i.bak "s/your-super-secure-jwt-secret-key-change-this-in-production/$JWT_SECRET/" .env
    
    print_status "Generated JWT secret"
    
    # Prompt for R2 configuration
    echo ""
    print_header "Cloudflare R2 Configuration (optional for development)"
    read -p "Do you want to configure Cloudflare R2 storage? (y/n): " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        read -p "R2 Access Key: " R2_ACCESS_KEY
        read -p "R2 Secret Key: " R2_SECRET_KEY
        read -p "R2 Endpoint (e.g., https://your-account.r2.cloudflarestorage.com): " R2_ENDPOINT
        read -p "R2 Bucket Name: " R2_BUCKET
        
        sed -i.bak "s/your-r2-access-key/$R2_ACCESS_KEY/" .env
        sed -i.bak "s/your-r2-secret-key/$R2_SECRET_KEY/" .env
        sed -i.bak "s|https://your-account-id.r2.cloudflarestorage.com|$R2_ENDPOINT|" .env
        sed -i.bak "s/cowcow-recordings/$R2_BUCKET/" .env
        
        print_status "R2 configuration saved"
    else
        print_status "Skipping R2 configuration. Using MinIO for local development."
    fi
    
    # Clean up backup files
    rm -f .env.bak
}

# Build and start services
start_development() {
    print_header "Starting CowCow in development mode..."
    
    # Enable development profile and MinIO
    export COMPOSE_PROFILES=development
    
    # Build and start services
    docker-compose -f docker-compose.yml -f docker-compose.dev.yml up --build -d
    
    print_status "Development environment started!"
    print_status "Services available at:"
    echo "  - API Server: http://localhost:8000"
    echo "  - API Docs: http://localhost:8000/docs"
    echo "  - MinIO Console: http://localhost:9001 (minioadmin/minioadmin123)"
    echo "  - PgAdmin: http://localhost:5050 (admin@cowcow.local/admin123)"
    echo "  - Redis Commander: http://localhost:8081"
}

# Start production
start_production() {
    print_header "Starting CowCow in production mode..."
    
    # Check for production requirements
    if [ ! -f nginx/ssl/server.crt ] || [ ! -f nginx/ssl/server.key ]; then
        print_warning "SSL certificates not found. Generating self-signed certificates..."
        mkdir -p nginx/ssl
        openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
            -keyout nginx/ssl/server.key \
            -out nginx/ssl/server.crt \
            -subj "/C=US/ST=State/L=City/O=Organization/CN=localhost"
        print_status "Self-signed certificates generated"
    fi
    
    # Create secrets
    echo "cowcow_password" | docker secret create postgres_password - 2>/dev/null || true
    
    # Enable production profile
    export COMPOSE_PROFILES=production
    
    # Start services
    docker-compose -f docker-compose.yml -f docker-compose.prod.yml up --build -d
    
    print_status "Production environment started!"
    print_status "Services available at:"
    echo "  - API Server (HTTPS): https://localhost"
    echo "  - API Docs: https://localhost/docs"
}

# Stop services
stop_services() {
    print_header "Stopping CowCow services..."
    
    docker-compose -f docker-compose.yml -f docker-compose.dev.yml down
    docker-compose -f docker-compose.yml -f docker-compose.prod.yml down
    
    print_status "Services stopped"
}

# Show logs
show_logs() {
    print_header "Showing service logs..."
    docker-compose logs -f
}

# Show status
show_status() {
    print_header "Service status:"
    docker-compose ps
}

# Clean up
cleanup() {
    print_header "Cleaning up Docker resources..."
    
    read -p "This will remove all containers, volumes, and images. Are you sure? (y/n): " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        docker-compose -f docker-compose.yml -f docker-compose.dev.yml down -v --rmi all
        docker-compose -f docker-compose.yml -f docker-compose.prod.yml down -v --rmi all
        print_status "Cleanup completed"
    else
        print_status "Cleanup cancelled"
    fi
}

# Main menu
show_menu() {
    echo ""
    print_header "CowCow Docker Management"
    echo "1. Setup environment and start development"
    echo "2. Start production"
    echo "3. Stop all services"
    echo "4. Show logs"
    echo "5. Show status"
    echo "6. Cleanup (remove all containers and volumes)"
    echo "7. Exit"
    echo ""
}

# Main script
main() {
    check_dependencies
    
    if [ $# -eq 0 ]; then
        # Interactive mode
        while true; do
            show_menu
            read -p "Select an option (1-7): " choice
            case $choice in
                1)
                    setup_environment
                    start_development
                    ;;
                2)
                    setup_environment
                    start_production
                    ;;
                3)
                    stop_services
                    ;;
                4)
                    show_logs
                    ;;
                5)
                    show_status
                    ;;
                6)
                    cleanup
                    ;;
                7)
                    print_status "Goodbye!"
                    exit 0
                    ;;
                *)
                    print_error "Invalid option. Please try again."
                    ;;
            esac
        done
    else
        # Command line mode
        case $1 in
            "dev"|"development")
                setup_environment
                start_development
                ;;
            "prod"|"production")
                setup_environment
                start_production
                ;;
            "stop")
                stop_services
                ;;
            "logs")
                show_logs
                ;;
            "status")
                show_status
                ;;
            "cleanup")
                cleanup
                ;;
            *)
                echo "Usage: $0 [dev|prod|stop|logs|status|cleanup]"
                exit 1
                ;;
        esac
    fi
}

# Run main function
main "$@"