import duckdb

conn = duckdb.connect('market_data/market_data.duckdb', read_only=True)

print('=== TESTING FINAL ARBITRAGE QUERY ===')
final_query = '''
WITH ohlcv_spreads AS (
    SELECT
        cb.timestamp,
        to_timestamp(cb.timestamp) as datetime,
        cb.close as coinbase_price,
        kr.close as kraken_price,
        ABS(cb.close - kr.close) as spread,
        cb.close - kr.close as price_diff,
        ABS(cb.close - kr.close) / LEAST(cb.close, kr.close) * 100 as spread_pct,
        CASE 
            WHEN cb.close > kr.close THEN 'Buy Kraken, Sell Coinbase'
            WHEN kr.close > cb.close THEN 'Buy Coinbase, Sell Kraken'
            ELSE 'No Opportunity'
        END as direction,
        cb.volume as cb_volume,
        kr.volume as kr_volume
    FROM ohlcv cb
    JOIN ohlcv kr ON cb.timestamp = kr.timestamp AND cb.symbol = kr.symbol
    WHERE cb.exchange = 'coinbase' 
        AND kr.exchange = 'kraken'
        AND cb.symbol = 'BTC/USD'
    ORDER BY cb.timestamp DESC
    LIMIT 5
)
SELECT * FROM ohlcv_spreads
'''

try:
    result = conn.execute(final_query).df()
    print(f'SUCCESS! Got {len(result)} synchronized candles')
    print(result)
    
    if len(result) > 0:
        print()
        print(f'Avg spread: ${result["spread"].mean():.2f}')
        print(f'Max spread: ${result["spread"].max():.2f}')
        print(f'Min spread: ${result["spread"].min():.2f}')
        
except Exception as e:
    print(f'ERROR: {e}')

conn.close()