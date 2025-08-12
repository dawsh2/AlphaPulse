#!/usr/bin/env python3
"""
PostgreSQL + TimescaleDB Setup for AlphaPulse
Alternative to DuckDB for better concurrency
"""

import psycopg2
import pandas as pd
import os
from typing import List, Dict, Any
import logging

logger = logging.getLogger(__name__)

class PostgreSQLSetup:
    """Setup PostgreSQL with TimescaleDB for trading data"""
    
    def __init__(self, 
                 host='localhost',
                 port=5432, 
                 database='alphapulse',
                 user='alphapulse',
                 password='alphapulse'):
        self.conn_params = {
            'host': host,
            'port': port,
            'database': database,
            'user': user,
            'password': password
        }
    
    def install_instructions(self):
        """Print installation instructions"""
        print("""
=== PostgreSQL + TimescaleDB Installation ===

# macOS (using Homebrew)
brew install postgresql timescaledb

# Start PostgreSQL
brew services start postgresql

# Create database and user
createdb alphapulse
createuser --interactive alphapulse

# Enable TimescaleDB
psql -d alphapulse -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"

# Python dependencies
pip install psycopg2-binary asyncpg

=== Alternative: Docker Setup ===
docker run -d --name timescaledb \\
  -p 5432:5432 \\
  -e POSTGRES_DB=alphapulse \\
  -e POSTGRES_USER=alphapulse \\
  -e POSTGRES_PASSWORD=alphapulse \\
  timescale/timescaledb:latest-pg15

=== Configuration ===
Add to your environment:
export POSTGRES_HOST=localhost
export POSTGRES_PORT=5432
export POSTGRES_DB=alphapulse
export POSTGRES_USER=alphapulse
export POSTGRES_PASSWORD=alphapulse
        """)
    
    def setup_schema(self):
        """Create tables and hypertables"""
        try:
            conn = psycopg2.connect(**self.conn_params)
            cur = conn.cursor()
            
            # Enable TimescaleDB
            cur.execute("CREATE EXTENSION IF NOT EXISTS timescaledb;")
            
            # Trades table with proper indexing
            cur.execute("""
                CREATE TABLE IF NOT EXISTS trades (
                    timestamp TIMESTAMPTZ NOT NULL,
                    datetime TIMESTAMPTZ NOT NULL,
                    symbol TEXT NOT NULL,
                    exchange TEXT NOT NULL,
                    price DECIMAL NOT NULL,
                    size DECIMAL NOT NULL,
                    side TEXT NOT NULL,
                    trade_id TEXT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    PRIMARY KEY (exchange, trade_id, timestamp)
                );
            """)
            
            # Convert to hypertable (if not already)
            try:
                cur.execute("""
                    SELECT create_hypertable('trades', 'timestamp', 
                                            chunk_time_interval => INTERVAL '1 hour',
                                            if_not_exists => TRUE);
                """)
            except psycopg2.Error as e:
                if "already a hypertable" not in str(e):
                    raise
            
            # Create indexes for common queries
            cur.execute("""
                CREATE INDEX IF NOT EXISTS idx_trades_exchange_time 
                ON trades (exchange, timestamp DESC);
            """)
            
            cur.execute("""
                CREATE INDEX IF NOT EXISTS idx_trades_symbol_time 
                ON trades (symbol, timestamp DESC);
            """)
            
            # OHLCV materialized view (continuous aggregate)
            cur.execute("""
                CREATE MATERIALIZED VIEW IF NOT EXISTS ohlcv_1m
                WITH (timescaledb.continuous) AS
                SELECT
                    time_bucket('1 minute', timestamp) AS time,
                    exchange,
                    symbol,
                    FIRST(price, timestamp) AS open,
                    MAX(price) AS high,
                    MIN(price) AS low,
                    LAST(price, timestamp) AS close,
                    SUM(size) AS volume,
                    COUNT(*) AS trade_count
                FROM trades
                GROUP BY time, exchange, symbol
                WITH NO DATA;
            """)
            
            # Refresh policy for continuous aggregate
            cur.execute("""
                SELECT add_continuous_aggregate_policy('ohlcv_1m',
                    start_offset => INTERVAL '2 minutes',
                    end_offset => INTERVAL '1 minute',
                    schedule_interval => INTERVAL '1 minute',
                    if_not_exists => TRUE);
            """)
            
            # Compression policy (compress data older than 1 day)
            cur.execute("""
                ALTER TABLE trades SET (
                    timescaledb.compress,
                    timescaledb.compress_segmentby = 'exchange,symbol'
                );
            """)
            
            cur.execute("""
                SELECT add_compression_policy('trades', INTERVAL '1 day', if_not_exists => TRUE);
            """)
            
            # Retention policy (keep data for 30 days)
            cur.execute("""
                SELECT add_retention_policy('trades', INTERVAL '30 days', if_not_exists => TRUE);
            """)
            
            conn.commit()
            cur.close()
            conn.close()
            
            logger.info("PostgreSQL schema setup complete")
            print("✅ PostgreSQL + TimescaleDB schema created successfully")
            
        except Exception as e:
            logger.error(f"Schema setup failed: {e}")
            raise
    
    def test_connection(self):
        """Test database connection and performance"""
        try:
            conn = psycopg2.connect(**self.conn_params)
            cur = conn.cursor()
            
            # Test basic query
            cur.execute("SELECT version();")
            version = cur.fetchone()[0]
            print(f"✅ Connected to: {version}")
            
            # Test TimescaleDB
            cur.execute("SELECT default_version, installed_version FROM pg_available_extensions WHERE name='timescaledb';")
            result = cur.fetchone()
            if result:
                print(f"✅ TimescaleDB: {result[1]} installed")
            else:
                print("❌ TimescaleDB not available")
            
            # Test insert performance
            import time
            test_data = [{
                'timestamp': '2025-08-11 17:00:00+00',
                'datetime': '2025-08-11 17:00:00+00',
                'symbol': 'BTC/USD',
                'exchange': 'test',
                'price': 60000.0,
                'size': 0.1,
                'side': 'buy',
                'trade_id': f'test_{i}'
            } for i in range(1000)]
            
            start_time = time.time()
            
            for trade in test_data:
                cur.execute("""
                    INSERT INTO trades (timestamp, datetime, symbol, exchange, price, size, side, trade_id)
                    VALUES (%(timestamp)s, %(datetime)s, %(symbol)s, %(exchange)s, 
                            %(price)s, %(size)s, %(side)s, %(trade_id)s)
                    ON CONFLICT (exchange, trade_id, timestamp) DO NOTHING;
                """, trade)
            
            conn.commit()
            insert_time = time.time() - start_time
            
            print(f"✅ Performance test: 1,000 inserts in {insert_time:.2f}s ({1000/insert_time:.0f} inserts/sec)")
            
            # Cleanup test data
            cur.execute("DELETE FROM trades WHERE exchange = 'test';")
            conn.commit()
            
            cur.close()
            conn.close()
            
        except Exception as e:
            print(f"❌ Connection test failed: {e}")
            raise

def main():
    """Setup wizard"""
    setup = PostgreSQLSetup()
    
    print("=== PostgreSQL + TimescaleDB Setup Wizard ===")
    print()
    
    choice = input("1) Show installation instructions\n2) Setup schema\n3) Test connection\n\nChoice: ")
    
    if choice == "1":
        setup.install_instructions()
    elif choice == "2":
        setup.setup_schema()
    elif choice == "3":
        setup.test_connection()
    else:
        print("Invalid choice")

if __name__ == "__main__":
    main()