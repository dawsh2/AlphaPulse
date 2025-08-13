#!/bin/bash
set -e

# Create multiple databases for different services
# This script is executed by the postgres Docker container on startup

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- Create market_data database for TimescaleDB
    CREATE DATABASE market_data;
    GRANT ALL PRIVILEGES ON DATABASE market_data TO $POSTGRES_USER;
    
    -- Create auth database for authentication service
    CREATE DATABASE auth;
    GRANT ALL PRIVILEGES ON DATABASE auth TO $POSTGRES_USER;
    
    -- Create nautilus database for trading engine
    CREATE DATABASE nautilus;
    GRANT ALL PRIVILEGES ON DATABASE nautilus TO $POSTGRES_USER;
EOSQL

# Initialize TimescaleDB in market_data database
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "market_data" <<-EOSQL
    CREATE EXTENSION IF NOT EXISTS timescaledb;
EOSQL

echo "Multiple databases created successfully"