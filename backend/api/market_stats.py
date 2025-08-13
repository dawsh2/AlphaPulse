"""
Market Data Stats API endpoint

⚠️ DEPRECATED - This file will be moved to Rust/Tokio services
   See: /rust-migration.md Phase 2
   Do not add new features or migrate to FastAPI
"""
from flask import Blueprint, jsonify, request, make_response
import psycopg2
from datetime import datetime, timedelta

market_stats_bp = Blueprint('market_stats', __name__)

def get_db_connection():
    """Get PostgreSQL connection"""
    return psycopg2.connect(
        host='localhost',
        port=5432,
        database='market_data',
        user='daws'
    )

@market_stats_bp.route('/api/market-data/stats', methods=['GET', 'OPTIONS'])
def get_market_stats():
    """Get market data statistics"""
    # Handle CORS preflight
    if request.method == 'OPTIONS':
        response = make_response()
        response.headers.add('Access-Control-Allow-Origin', '*')
        response.headers.add('Access-Control-Allow-Headers', 'Content-Type')
        response.headers.add('Access-Control-Allow-Methods', 'GET, OPTIONS')
        return response
    
    try:
        conn = get_db_connection()
        cur = conn.cursor()
        
        # Get total trades in last hour
        cur.execute("""
            SELECT COUNT(*) 
            FROM trades 
            WHERE time > NOW() - INTERVAL '1 hour'
        """)
        total_trades = cur.fetchone()[0]
        
        # Get latest BTC price
        cur.execute("""
            SELECT price 
            FROM trades 
            WHERE symbol IN ('BTC-USD', 'BTC/USD')
            ORDER BY time DESC 
            LIMIT 1
        """)
        result = cur.fetchone()
        btc_price = result[0] if result else None
        
        # Get latest ETH price
        cur.execute("""
            SELECT price 
            FROM trades 
            WHERE symbol IN ('ETH-USD', 'ETH/USD')
            ORDER BY time DESC 
            LIMIT 1
        """)
        result = cur.fetchone()
        eth_price = result[0] if result else None
        
        # Get recent trades
        cur.execute("""
            SELECT time, exchange, symbol, price, size, side
            FROM trades
            ORDER BY time DESC
            LIMIT 20
        """)
        
        recent_trades = []
        for row in cur.fetchall():
            recent_trades.append({
                'time': row[0].isoformat(),
                'exchange': row[1],
                'symbol': row[2],
                'price': float(row[3]),
                'size': float(row[4]),
                'side': row[5] or 'unknown'
            })
        
        # Get exchange stats
        cur.execute("""
            SELECT 
                exchange,
                COUNT(*) as total_trades,
                COUNT(*) / NULLIF(EXTRACT(EPOCH FROM (MAX(time) - MIN(time))), 0) as trades_per_second,
                MAX(time) as last_trade_time
            FROM trades
            WHERE time > NOW() - INTERVAL '1 hour'
            GROUP BY exchange
        """)
        
        exchanges = []
        for row in cur.fetchall():
            exchanges.append({
                'exchange': row[0],
                'total_trades': row[1],
                'trades_per_second': float(row[2]) if row[2] else 0,
                'last_trade_time': row[3].isoformat() if row[3] else None
            })
        
        cur.close()
        conn.close()
        
        response = jsonify({
            'total_trades': total_trades,
            'btc_price': float(btc_price) if btc_price else None,
            'eth_price': float(eth_price) if eth_price else None,
            'recent_trades': recent_trades,
            'exchanges': exchanges
        })
        
        # Add CORS headers
        response.headers.add('Access-Control-Allow-Origin', '*')
        response.headers.add('Access-Control-Allow-Headers', 'Content-Type')
        
        return response
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500