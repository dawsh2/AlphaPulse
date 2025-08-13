#!/bin/bash

# AlphaPulse Market Data Infrastructure Startup Script

set -e

echo "========================================="
echo "AlphaPulse Market Data Infrastructure"
echo "========================================="

# Check for required environment variables
check_env_vars() {
    local missing_vars=()
    
    # Optional but recommended
    if [ -z "$ALPACA_API_KEY" ]; then
        echo "⚠️  Warning: ALPACA_API_KEY not set"
    fi
    
    if [ -z "$ALPACA_API_SECRET" ]; then
        echo "⚠️  Warning: ALPACA_API_SECRET not set"
    fi
    
    echo ""
}

# Function to start services
start_services() {
    echo "Starting market data infrastructure..."
    
    # Start core services (TimescaleDB, Redis)
    echo "1. Starting core services (TimescaleDB, Redis)..."
    docker-compose up -d timescaledb redis
    
    # Wait for services to be healthy
    echo "2. Waiting for database to be ready..."
    sleep 10
    
    # Start data orchestrator
    echo "3. Starting data orchestrator..."
    docker-compose up -d data-orchestrator
    
    # Start monitoring (optional)
    if [ "$1" == "--with-monitoring" ]; then
        echo "4. Starting monitoring services..."
        docker-compose up -d prometheus grafana
    fi
    
    # Start development tools (optional)
    if [ "$1" == "--dev" ] || [ "$2" == "--dev" ]; then
        echo "5. Starting development tools..."
        docker-compose --profile dev up -d
    fi
    
    echo ""
    echo "✅ Market data infrastructure started successfully!"
    echo ""
    echo "Services available at:"
    echo "  - TimescaleDB: localhost:5432"
    echo "  - Redis: localhost:6379"
    
    if [ "$1" == "--with-monitoring" ] || [ "$2" == "--with-monitoring" ]; then
        echo "  - Prometheus: http://localhost:9090"
        echo "  - Grafana: http://localhost:3000 (admin/admin)"
    fi
    
    if [ "$1" == "--dev" ] || [ "$2" == "--dev" ]; then
        echo "  - pgAdmin: http://localhost:5050"
        echo "  - Redis Commander: http://localhost:8081"
    fi
    
    echo ""
    echo "To view logs: docker-compose logs -f data-orchestrator"
    echo "To stop: ./stop-data-infra.sh"
}

# Function to check Docker
check_docker() {
    if ! command -v docker &> /dev/null; then
        echo "❌ Docker is not installed. Please install Docker first."
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        echo "❌ Docker Compose is not installed. Please install Docker Compose first."
        exit 1
    fi
    
    # Check if Docker daemon is running
    if ! docker info > /dev/null 2>&1; then
        echo "❌ Docker daemon is not running. Please start Docker."
        exit 1
    fi
}

# Main execution
main() {
    check_docker
    check_env_vars
    
    # Parse arguments
    case "$1" in
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --with-monitoring  Start with Prometheus and Grafana"
            echo "  --dev             Start with development tools (pgAdmin, Redis Commander)"
            echo "  --help            Show this help message"
            echo ""
            exit 0
            ;;
        *)
            start_services "$@"
            ;;
    esac
}

# Run main function
main "$@"