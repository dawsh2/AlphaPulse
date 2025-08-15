-- TimescaleDB Schema for AlphaPulse Data Writer
-- Creates hypertables optimized for time-series market data storage

-- Create database if not exists
-- This should be run by an admin user
-- CREATE DATABASE market_data;

-- Connect to market_data database
\c market_data;

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Create schema for market data
CREATE SCHEMA IF NOT EXISTS market_data;

-- =============================================================================
-- TRADES TABLE
-- =============================================================================

CREATE TABLE IF NOT EXISTS market_data.trades (
    time TIMESTAMPTZ NOT NULL,
    exchange TEXT NOT NULL,
    symbol_hash BIGINT NOT NULL,
    symbol TEXT NOT NULL,
    price DECIMAL(20,8) NOT NULL,
    volume DECIMAL(20,8) NOT NULL,
    side TEXT NOT NULL CHECK (side IN ('buy', 'sell', 'unknown'))
);

-- Convert to hypertable (must be done immediately after table creation)
SELECT create_hypertable(
    'market_data.trades', 
    'time',
    if_not_exists => TRUE,
    chunk_time_interval => INTERVAL '1 hour'
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_trades_symbol_time 
    ON market_data.trades (symbol_hash, time DESC);

CREATE INDEX IF NOT EXISTS idx_trades_exchange_time 
    ON market_data.trades (exchange, time DESC);

CREATE INDEX IF NOT EXISTS idx_trades_symbol_name_time 
    ON market_data.trades (symbol, time DESC);

-- =============================================================================
-- L2_DELTAS TABLE  
-- =============================================================================

CREATE TABLE IF NOT EXISTS market_data.l2_deltas (
    time TIMESTAMPTZ NOT NULL,
    exchange TEXT NOT NULL,
    symbol_hash BIGINT NOT NULL,
    symbol TEXT NOT NULL,
    sequence BIGINT NOT NULL,
    updates JSONB NOT NULL
);

-- Convert to hypertable
SELECT create_hypertable(
    'market_data.l2_deltas', 
    'time',
    if_not_exists => TRUE,
    chunk_time_interval => INTERVAL '1 hour'
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_l2_deltas_symbol_time 
    ON market_data.l2_deltas (symbol_hash, time DESC);

CREATE INDEX IF NOT EXISTS idx_l2_deltas_exchange_time 
    ON market_data.l2_deltas (exchange, time DESC);

CREATE INDEX IF NOT EXISTS idx_l2_deltas_symbol_name_time 
    ON market_data.l2_deltas (symbol, time DESC);

CREATE INDEX IF NOT EXISTS idx_l2_deltas_sequence 
    ON market_data.l2_deltas (exchange, symbol_hash, sequence);

-- JSONB index for updates field (for querying delta contents)
CREATE INDEX IF NOT EXISTS idx_l2_deltas_updates_gin 
    ON market_data.l2_deltas USING gin (updates);

-- =============================================================================
-- COMPRESSION POLICIES
-- =============================================================================

-- Enable compression for older data to save storage space
-- Compress trades data older than 7 days
SELECT add_compression_policy(
    'market_data.trades', 
    INTERVAL '7 days',
    if_not_exists => TRUE
);

-- Compress L2 deltas older than 3 days (more frequent due to higher volume)
SELECT add_compression_policy(
    'market_data.l2_deltas', 
    INTERVAL '3 days',
    if_not_exists => TRUE
);

-- =============================================================================
-- RETENTION POLICIES
-- =============================================================================

-- Drop old chunks after specified time (configurable per deployment)
-- Keep trades for 1 year in TimescaleDB
SELECT add_retention_policy(
    'market_data.trades', 
    INTERVAL '365 days',
    if_not_exists => TRUE
);

-- Keep L2 deltas for 90 days in TimescaleDB (export to Parquet for longer term)
SELECT add_retention_policy(
    'market_data.l2_deltas', 
    INTERVAL '90 days',
    if_not_exists => TRUE
);

-- =============================================================================
-- CONTINUOUS AGGREGATES (for common queries)
-- =============================================================================

-- 1-minute OHLCV aggregation for trades
CREATE MATERIALIZED VIEW IF NOT EXISTS market_data.trades_1min
WITH (timescaledb.continuous) AS
SELECT 
    time_bucket('1 minute', time) AS bucket,
    exchange,
    symbol_hash,
    symbol,
    first(price, time) AS open,
    max(price) AS high,
    min(price) AS low,
    last(price, time) AS close,
    sum(volume) AS volume,
    count(*) AS trade_count
FROM market_data.trades
GROUP BY bucket, exchange, symbol_hash, symbol;

-- Add refresh policy for continuous aggregate
SELECT add_continuous_aggregate_policy(
    'market_data.trades_1min',
    start_offset => INTERVAL '1 hour',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute',
    if_not_exists => TRUE
);

-- Hourly OHLCV aggregation
CREATE MATERIALIZED VIEW IF NOT EXISTS market_data.trades_1hour
WITH (timescaledb.continuous) AS
SELECT 
    time_bucket('1 hour', time) AS bucket,
    exchange,
    symbol_hash,
    symbol,
    first(price, time) AS open,
    max(price) AS high,
    min(price) AS low,
    last(price, time) AS close,
    sum(volume) AS volume,
    count(*) AS trade_count
FROM market_data.trades
GROUP BY bucket, exchange, symbol_hash, symbol;

-- Add refresh policy for hourly aggregate
SELECT add_continuous_aggregate_policy(
    'market_data.trades_1hour',
    start_offset => INTERVAL '1 day',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour',
    if_not_exists => TRUE
);

-- =============================================================================
-- SYMBOL MAPPINGS TABLE (for hash resolution)
-- =============================================================================

CREATE TABLE IF NOT EXISTS market_data.symbol_mappings (
    symbol_hash BIGINT PRIMARY KEY,
    exchange TEXT NOT NULL,
    symbol TEXT NOT NULL,
    full_symbol TEXT NOT NULL,  -- exchange:symbol format
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Insert known symbol mappings (from data writer implementation)
INSERT INTO market_data.symbol_mappings (symbol_hash, exchange, symbol, full_symbol) VALUES
    (16842681295735137662, 'coinbase', 'BTC-USD', 'coinbase:BTC-USD'),
    (7334401999635196894, 'coinbase', 'ETH-USD', 'coinbase:ETH-USD'),
    (940696374048161387, 'coinbase', 'SOL-USD', 'coinbase:SOL-USD'),
    (2928176905300374322, 'coinbase', 'LINK-USD', 'coinbase:LINK-USD'),
    (1022169821381239205, 'kraken', 'BTC-USD', 'kraken:BTC-USD'),
    (6206069765414077566, 'kraken', 'ETH-USD', 'kraken:ETH-USD')
ON CONFLICT (symbol_hash) DO UPDATE SET
    updated_at = NOW();

-- =============================================================================
-- PERMISSIONS
-- =============================================================================

-- Create role for data writer service
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'alphapulse_writer') THEN
        CREATE ROLE alphapulse_writer WITH LOGIN PASSWORD 'secure_password_change_me';
    END IF;
END
$$;

-- Grant necessary permissions
GRANT USAGE ON SCHEMA market_data TO alphapulse_writer;
GRANT INSERT, SELECT ON ALL TABLES IN SCHEMA market_data TO alphapulse_writer;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA market_data TO alphapulse_writer;

-- Create role for read-only access (for analytics, frontend)
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'alphapulse_reader') THEN
        CREATE ROLE alphapulse_reader WITH LOGIN PASSWORD 'reader_password_change_me';
    END IF;
END
$$;

-- Grant read-only permissions
GRANT USAGE ON SCHEMA market_data TO alphapulse_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA market_data TO alphapulse_reader;

-- =============================================================================
-- UTILITY FUNCTIONS
-- =============================================================================

-- Function to get recent trades for a symbol
CREATE OR REPLACE FUNCTION market_data.get_recent_trades(
    p_symbol_hash BIGINT,
    p_limit INTEGER DEFAULT 100
)
RETURNS TABLE (
    time TIMESTAMPTZ,
    exchange TEXT,
    symbol TEXT,
    price DECIMAL(20,8),
    volume DECIMAL(20,8),
    side TEXT
) AS $$
BEGIN
    RETURN QUERY
    SELECT t.time, t.exchange, t.symbol, t.price, t.volume, t.side
    FROM market_data.trades t
    WHERE t.symbol_hash = p_symbol_hash
    ORDER BY t.time DESC
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- Function to rebuild L2 orderbook from deltas
CREATE OR REPLACE FUNCTION market_data.rebuild_orderbook(
    p_symbol_hash BIGINT,
    p_from_time TIMESTAMPTZ,
    p_to_time TIMESTAMPTZ DEFAULT NOW()
)
RETURNS TABLE (
    side TEXT,
    price DECIMAL(20,8),
    size DECIMAL(20,8)
) AS $$
DECLARE
    delta_record RECORD;
    update_record RECORD;
BEGIN
    -- This is a simplified version - full implementation would need
    -- to properly apply all delta updates in sequence
    
    FOR delta_record IN
        SELECT l.updates
        FROM market_data.l2_deltas l
        WHERE l.symbol_hash = p_symbol_hash
          AND l.time >= p_from_time
          AND l.time <= p_to_time
        ORDER BY l.time, l.sequence
    LOOP
        -- Process each update in the delta
        -- This would need proper JSONB processing to extract
        -- bids/asks arrays and apply updates
        NULL;
    END LOOP;
    
    RETURN;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- MONITORING VIEWS
-- =============================================================================

-- View for monitoring data ingestion rates
CREATE OR REPLACE VIEW market_data.ingestion_stats AS
SELECT 
    'trades' as table_name,
    exchange,
    symbol,
    count(*) as message_count,
    min(time) as first_message,
    max(time) as last_message,
    extract(epoch from (max(time) - min(time)))/60 as duration_minutes,
    count(*) / GREATEST(extract(epoch from (max(time) - min(time)))/60, 1) as messages_per_minute
FROM market_data.trades
WHERE time >= NOW() - INTERVAL '1 hour'
GROUP BY exchange, symbol

UNION ALL

SELECT 
    'l2_deltas' as table_name,
    exchange,
    symbol,
    count(*) as message_count,
    min(time) as first_message,
    max(time) as last_message,
    extract(epoch from (max(time) - min(time)))/60 as duration_minutes,
    count(*) / GREATEST(extract(epoch from (max(time) - min(time)))/60, 1) as messages_per_minute
FROM market_data.l2_deltas
WHERE time >= NOW() - INTERVAL '1 hour'
GROUP BY exchange, symbol;

-- View for checking data gaps
CREATE OR REPLACE VIEW market_data.data_gaps AS
WITH time_series AS (
    SELECT generate_series(
        date_trunc('minute', NOW() - INTERVAL '1 hour'),
        date_trunc('minute', NOW()),
        INTERVAL '1 minute'
    ) AS minute_bucket
),
actual_data AS (
    SELECT 
        date_trunc('minute', time) AS minute_bucket,
        exchange,
        symbol,
        count(*) as message_count
    FROM market_data.trades
    WHERE time >= NOW() - INTERVAL '1 hour'
    GROUP BY date_trunc('minute', time), exchange, symbol
)
SELECT 
    ts.minute_bucket,
    COALESCE(ad.exchange, 'missing') as exchange,
    COALESCE(ad.symbol, 'missing') as symbol,
    COALESCE(ad.message_count, 0) as message_count
FROM time_series ts
LEFT JOIN actual_data ad ON ts.minute_bucket = ad.minute_bucket
WHERE ad.message_count IS NULL OR ad.message_count = 0
ORDER BY ts.minute_bucket DESC;

-- =============================================================================
-- INITIAL SETUP COMPLETE
-- =============================================================================

-- Display schema information
\dt market_data.*
\di market_data.*

-- Show hypertable information
SELECT hypertable_name, num_chunks 
FROM timescaledb_information.hypertables 
WHERE hypertable_schema = 'market_data';

NOTIFY data_writer_schema, 'TimescaleDB schema setup completed successfully';