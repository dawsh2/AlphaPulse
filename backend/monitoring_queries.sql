-- SYSTEM HEALTH MONITORING QUERIES FOR GRAFANA

-- 1. EXCHANGE CONNECTION STATUS (Shows if streams are alive)
-- Use in a Table panel
SELECT 
    exchange,
    COUNT(*) as trades_last_minute,
    MAX(time) as last_trade_time,
    EXTRACT(EPOCH FROM (NOW() - MAX(time))) as seconds_since_last_trade,
    CASE 
        WHEN MAX(time) > NOW() - INTERVAL '30 seconds' THEN '🟢 LIVE'
        WHEN MAX(time) > NOW() - INTERVAL '60 seconds' THEN '🟡 DELAYED'
        ELSE '🔴 DISCONNECTED'
    END as status
FROM trades 
WHERE time > NOW() - INTERVAL '5 minutes'
GROUP BY exchange
ORDER BY exchange;

-- 2. TRADES PER SECOND (Real-time ingestion rate)
-- Use in a Gauge panel
SELECT 
    COUNT(*)::float / 60 as trades_per_second
FROM trades 
WHERE time > NOW() - INTERVAL '1 minute';

-- 3. SYMBOL COVERAGE (Which symbols are we receiving)
-- Use in a Table panel  
SELECT 
    exchange,
    symbol,
    COUNT(*) as trades_today,
    MAX(time) as last_update,
    CASE 
        WHEN MAX(time) > NOW() - INTERVAL '1 minute' THEN 'ACTIVE'
        ELSE 'INACTIVE'
    END as status
FROM trades
WHERE time > NOW() - INTERVAL '1 hour'
GROUP BY exchange, symbol
ORDER BY exchange, symbol;

-- 4. DATA LAG MONITORING (Are we getting real-time data?)
-- Use in a Stat panel
SELECT 
    exchange,
    EXTRACT(EPOCH FROM (NOW() - MAX(time))) as lag_seconds
FROM trades
WHERE time > NOW() - INTERVAL '5 minutes'
GROUP BY exchange;

-- 5. INGESTION RATE OVER TIME (See patterns/drops)
-- Use in a Time Series panel
SELECT 
    date_trunc('minute', time) AS time,
    exchange,
    COUNT(*) as trades_per_minute
FROM trades
WHERE $__timeFilter(time)
GROUP BY 1, exchange
ORDER BY 1;

-- 6. MISSING DATA ALERTS
-- Use in an Alert panel
SELECT 
    exchange,
    symbol,
    'NO DATA FOR ' || EXTRACT(EPOCH FROM (NOW() - MAX(time)))::INT || ' SECONDS' as alert
FROM trades
WHERE time > NOW() - INTERVAL '10 minutes'
GROUP BY exchange, symbol
HAVING MAX(time) < NOW() - INTERVAL '2 minutes';

-- 7. DATABASE PERFORMANCE
-- Use in Stat panels
SELECT 
    pg_database_size('market_data')/1024/1024 as database_size_mb,
    (SELECT COUNT(*) FROM trades) as total_records,
    (SELECT COUNT(*) FROM trades WHERE time > NOW() - INTERVAL '1 hour') as records_last_hour;