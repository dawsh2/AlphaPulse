-- Initialize TimescaleDB extension and create market data schema
-- This script runs automatically when the Docker container starts

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Create schema for market data
CREATE SCHEMA IF NOT EXISTS market_data;

-- Set search path
SET search_path TO market_data, public;

-- Create trades table for tick data
CREATE TABLE IF NOT EXISTS trades (
    time TIMESTAMPTZ NOT NULL,
    exchange VARCHAR(50) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    size DOUBLE PRECISION NOT NULL,
    side VARCHAR(10) CHECK (side IN ('buy', 'sell', 'unknown')),
    trade_id VARCHAR(100),
    conditions TEXT[],
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Convert to hypertable with 1 day chunks
SELECT create_hypertable('trades', 'time', 
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_trades_exchange_symbol_time 
    ON trades (exchange, symbol, time DESC);
CREATE INDEX IF NOT EXISTS idx_trades_symbol_time 
    ON trades (symbol, time DESC);
CREATE INDEX IF NOT EXISTS idx_trades_trade_id 
    ON trades (exchange, trade_id) 
    WHERE trade_id IS NOT NULL;

-- Create OHLCV table for candlestick data
CREATE TABLE IF NOT EXISTS ohlcv (
    time TIMESTAMPTZ NOT NULL,
    exchange VARCHAR(50) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    interval VARCHAR(10) NOT NULL, -- '1m', '5m', '15m', '1h', '1d'
    open DOUBLE PRECISION NOT NULL,
    high DOUBLE PRECISION NOT NULL,
    low DOUBLE PRECISION NOT NULL,
    close DOUBLE PRECISION NOT NULL,
    volume DOUBLE PRECISION NOT NULL,
    trade_count INTEGER,
    vwap DOUBLE PRECISION,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(exchange, symbol, interval, time)
);

-- Convert to hypertable
SELECT create_hypertable('ohlcv', 'time',
    chunk_time_interval => INTERVAL '7 days',
    if_not_exists => TRUE
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_ohlcv_exchange_symbol_interval_time 
    ON ohlcv (exchange, symbol, interval, time DESC);

-- Create order book snapshots table
CREATE TABLE IF NOT EXISTS orderbook_snapshots (
    time TIMESTAMPTZ NOT NULL,
    exchange VARCHAR(50) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    side VARCHAR(10) CHECK (side IN ('bid', 'ask')),
    level INTEGER NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    size DOUBLE PRECISION NOT NULL,
    order_count INTEGER,
    snapshot_id VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Convert to hypertable with smaller chunks for order book data
SELECT create_hypertable('orderbook_snapshots', 'time',
    chunk_time_interval => INTERVAL '1 hour',
    if_not_exists => TRUE
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_orderbook_exchange_symbol_time 
    ON orderbook_snapshots (exchange, symbol, time DESC);
CREATE INDEX IF NOT EXISTS idx_orderbook_snapshot_id 
    ON orderbook_snapshots (snapshot_id) 
    WHERE snapshot_id IS NOT NULL;

-- Create continuous aggregates for common queries

-- 1-minute OHLCV from trades
CREATE MATERIALIZED VIEW IF NOT EXISTS ohlcv_1m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 minute', time) AS bucket,
    exchange,
    symbol,
    FIRST(price, time) AS open,
    MAX(price) AS high,
    MIN(price) AS low,
    LAST(price, time) AS close,
    SUM(size) AS volume,
    COUNT(*) AS trade_count,
    SUM(price * size) / NULLIF(SUM(size), 0) AS vwap
FROM trades
GROUP BY bucket, exchange, symbol
WITH NO DATA;

-- Refresh policy for 1-minute data
SELECT add_continuous_aggregate_policy('ohlcv_1m',
    start_offset => INTERVAL '2 hours',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute',
    if_not_exists => TRUE
);

-- 5-minute OHLCV
CREATE MATERIALIZED VIEW IF NOT EXISTS ohlcv_5m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('5 minutes', bucket) AS bucket,
    exchange,
    symbol,
    FIRST(open, bucket) AS open,
    MAX(high) AS high,
    MIN(low) AS low,
    LAST(close, bucket) AS close,
    SUM(volume) AS volume,
    SUM(trade_count) AS trade_count,
    SUM(vwap * volume) / NULLIF(SUM(volume), 0) AS vwap
FROM ohlcv_1m
GROUP BY time_bucket('5 minutes', bucket), exchange, symbol
WITH NO DATA;

-- Refresh policy for 5-minute data
SELECT add_continuous_aggregate_policy('ohlcv_5m',
    start_offset => INTERVAL '6 hours',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes',
    if_not_exists => TRUE
);

-- Data retention policies

-- Keep raw trades for 30 days
SELECT add_retention_policy('trades', 
    INTERVAL '30 days',
    if_not_exists => TRUE
);

-- Keep order book snapshots for 7 days
SELECT add_retention_policy('orderbook_snapshots',
    INTERVAL '7 days',
    if_not_exists => TRUE
);

-- Keep OHLCV data for 1 year
SELECT add_retention_policy('ohlcv',
    INTERVAL '365 days',
    if_not_exists => TRUE
);

-- Compression policies for older data

-- Compress trades older than 7 days
SELECT add_compression_policy('trades',
    INTERVAL '7 days',
    if_not_exists => TRUE
);

-- Compress OHLCV older than 30 days
SELECT add_compression_policy('ohlcv',
    INTERVAL '30 days',
    if_not_exists => TRUE
);

-- Create monitoring views

CREATE OR REPLACE VIEW data_health AS
SELECT
    'trades' AS table_name,
    COUNT(*) AS total_rows,
    MIN(time) AS oldest_record,
    MAX(time) AS newest_record,
    EXTRACT(EPOCH FROM (MAX(time) - MIN(time))) / 86400 AS days_of_data,
    COUNT(DISTINCT exchange) AS exchanges,
    COUNT(DISTINCT symbol) AS symbols
FROM trades
UNION ALL
SELECT
    'ohlcv' AS table_name,
    COUNT(*) AS total_rows,
    MIN(time) AS oldest_record,
    MAX(time) AS newest_record,
    EXTRACT(EPOCH FROM (MAX(time) - MIN(time))) / 86400 AS days_of_data,
    COUNT(DISTINCT exchange) AS exchanges,
    COUNT(DISTINCT symbol) AS symbols
FROM ohlcv;

-- Create function for getting latest prices
CREATE OR REPLACE FUNCTION get_latest_prices(
    p_symbols TEXT[] DEFAULT NULL
)
RETURNS TABLE (
    symbol VARCHAR,
    exchange VARCHAR,
    price DOUBLE PRECISION,
    size DOUBLE PRECISION,
    time TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT DISTINCT ON (t.symbol, t.exchange)
        t.symbol,
        t.exchange,
        t.price,
        t.size,
        t.time
    FROM trades t
    WHERE 
        (p_symbols IS NULL OR t.symbol = ANY(p_symbols))
        AND t.time > NOW() - INTERVAL '1 hour'
    ORDER BY t.symbol, t.exchange, t.time DESC;
END;
$$ LANGUAGE plpgsql;

-- Grant permissions
GRANT ALL PRIVILEGES ON SCHEMA market_data TO alphapulse;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA market_data TO alphapulse;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA market_data TO alphapulse;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA market_data TO alphapulse;