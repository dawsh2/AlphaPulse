#!/bin/bash

# AlphaPulse Market Data Infrastructure Shutdown Script

echo "========================================="
echo "Stopping AlphaPulse Data Infrastructure"
echo "========================================="

# Stop all services
echo "Stopping all services..."
docker-compose down

# Optional: Remove volumes (data)
if [ "$1" == "--clean" ]; then
    echo "Removing data volumes..."
    docker-compose down -v
    echo "⚠️  All data has been removed"
fi

echo "✅ Services stopped successfully"

# Show status
echo ""
echo "To restart: ./start-data-infra.sh"
echo "To remove all data: $0 --clean"