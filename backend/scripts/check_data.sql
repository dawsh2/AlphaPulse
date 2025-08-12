-- Check data balance between exchanges
SELECT 
    exchange,
    symbol,
    COUNT(*) as bar_count,
    datetime(MIN(timestamp), 'unixepoch') as first_bar,
    datetime(MAX(timestamp), 'unixepoch') as last_bar,
    ROUND((MAX(timestamp) - MIN(timestamp)) / 3600.0, 1) as hours_of_data
FROM ohlcv
WHERE symbol = 'BTC/USD'
GROUP BY exchange, symbol
ORDER BY bar_count DESC;

-- Check recent data (last 24 hours)
SELECT 
    exchange,
    COUNT(*) as bars_last_24h,
    datetime(MIN(timestamp), 'unixepoch') as oldest,
    datetime(MAX(timestamp), 'unixepoch') as newest
FROM ohlcv
WHERE symbol = 'BTC/USD'
  AND timestamp > unixepoch('now') - 86400
GROUP BY exchange;

-- Find gaps in Kraken data
WITH kraken_data AS (
    SELECT 
        timestamp,
        LAG(timestamp) OVER (ORDER BY timestamp) as prev_timestamp,
        timestamp - LAG(timestamp) OVER (ORDER BY timestamp) as gap_seconds
    FROM ohlcv
    WHERE exchange = 'kraken' AND symbol = 'BTC/USD'
)
SELECT 
    COUNT(*) as total_gaps,
    SUM(CASE WHEN gap_seconds > 120 THEN 1 ELSE 0 END) as large_gaps,
    MAX(gap_seconds) / 3600.0 as max_gap_hours
FROM kraken_data
WHERE gap_seconds IS NOT NULL;