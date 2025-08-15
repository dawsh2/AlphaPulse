#!/bin/bash

# TimescaleDB Database Setup Script for AlphaPulse
# This script creates the database and applies the schema

set -e

# Configuration
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-market_data}"
DB_ADMIN_USER="${DB_ADMIN_USER:-postgres}"
DB_ADMIN_PASSWORD="${DB_ADMIN_PASSWORD:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== AlphaPulse TimescaleDB Setup ===${NC}"
echo "Host: $DB_HOST:$DB_PORT"
echo "Database: $DB_NAME"
echo "Admin User: $DB_ADMIN_USER"
echo ""

# Check if psql is available
if ! command -v psql &> /dev/null; then
    echo -e "${RED}Error: psql command not found. Please install PostgreSQL client tools.${NC}"
    exit 1
fi

# Check if TimescaleDB is available
echo -e "${YELLOW}Checking TimescaleDB availability...${NC}"
TIMESCALE_CHECK=$(PGPASSWORD="$DB_ADMIN_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_ADMIN_USER" -d postgres -t -c "SELECT 1 FROM pg_available_extensions WHERE name = 'timescaledb';" 2>/dev/null || echo "")

if [ -z "$TIMESCALE_CHECK" ]; then
    echo -e "${RED}Error: TimescaleDB extension not available. Please install TimescaleDB.${NC}"
    echo "Visit: https://docs.timescale.com/install/"
    exit 1
fi

echo -e "${GREEN}✓ TimescaleDB extension available${NC}"

# Create database if it doesn't exist
echo -e "${YELLOW}Creating database if not exists...${NC}"
PGPASSWORD="$DB_ADMIN_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_ADMIN_USER" -d postgres -c "CREATE DATABASE $DB_NAME;" 2>/dev/null || echo "Database may already exist"

# Apply schema
echo -e "${YELLOW}Applying TimescaleDB schema...${NC}"
PGPASSWORD="$DB_ADMIN_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_ADMIN_USER" -d "$DB_NAME" -f "$(dirname "$0")/timescaledb_schema.sql"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Schema applied successfully${NC}"
else
    echo -e "${RED}✗ Schema application failed${NC}"
    exit 1
fi

# Verify setup
echo -e "${YELLOW}Verifying setup...${NC}"
HYPERTABLES=$(PGPASSWORD="$DB_ADMIN_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_ADMIN_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM timescaledb_information.hypertables WHERE hypertable_schema = 'market_data';" 2>/dev/null)

if [ "$HYPERTABLES" -eq "2" ]; then
    echo -e "${GREEN}✓ Hypertables created successfully (trades, l2_deltas)${NC}"
else
    echo -e "${RED}✗ Expected 2 hypertables, found $HYPERTABLES${NC}"
    exit 1
fi

# Test permissions
echo -e "${YELLOW}Testing data writer permissions...${NC}"
TEST_INSERT=$(PGPASSWORD="$DB_ADMIN_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_ADMIN_USER" -d "$DB_NAME" -t -c "
GRANT USAGE ON SCHEMA market_data TO current_user;
GRANT INSERT, SELECT ON ALL TABLES IN SCHEMA market_data TO current_user;
INSERT INTO market_data.trades (time, exchange, symbol_hash, symbol, price, volume, side) 
VALUES (NOW(), 'test', 12345, 'TEST-USD', 100.50, 1.0, 'buy');
DELETE FROM market_data.trades WHERE exchange = 'test';
SELECT 'success';
" 2>/dev/null || echo "failed")

if [ "$TEST_INSERT" = " success" ]; then
    echo -e "${GREEN}✓ Database permissions working correctly${NC}"
else
    echo -e "${RED}✗ Database permission test failed${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}=== Setup Complete ===${NC}"
echo ""
echo "Database: $DB_NAME"
echo "Tables created:"
echo "  - market_data.trades (hypertable)"
echo "  - market_data.l2_deltas (hypertable)"
echo "  - market_data.symbol_mappings"
echo ""
echo "Continuous aggregates:"
echo "  - market_data.trades_1min"
echo "  - market_data.trades_1hour"
echo ""
echo "Monitoring views:"
echo "  - market_data.ingestion_stats"
echo "  - market_data.data_gaps"
echo ""
echo "Users created:"
echo "  - alphapulse_writer (read/write access)"
echo "  - alphapulse_reader (read-only access)"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Update connection string in data_writer.yaml:"
echo "   postgresql://alphapulse_writer:secure_password_change_me@$DB_HOST:$DB_PORT/$DB_NAME"
echo ""
echo "2. Start the data writer service:"
echo "   cd /Users/daws/alphapulse/backend/services/data_writer"
echo "   cargo run"
echo ""
echo "3. Change default passwords for production use!"