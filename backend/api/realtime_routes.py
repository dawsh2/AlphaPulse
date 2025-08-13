"""
Real-time WebSocket routes for Flask app
Streams trades from PostgreSQL database via NOTIFY to frontend

⚠️ DEPRECATED - This file will be moved to Rust/Tokio services
   See: /rust-migration.md Phase 3
   Do not add new features or migrate to FastAPI
"""
from flask import Blueprint
from flask_socketio import SocketIO, emit
import psycopg2
import select
import json
import threading
from datetime import datetime
import time
import logging

realtime_bp = Blueprint('realtime', __name__)
socketio = None  # Will be initialized in app.py
logger = logging.getLogger(__name__)

class TradeStreamer:
    def __init__(self, socket_io, flask_app):
        self.socketio = socket_io
        self.flask_app = flask_app
        self.running = False
        self.clients = set()
        self.conn = None
        self.thread = None
        
    def start(self):
        """Start streaming from PostgreSQL database"""
        if self.running:
            return
            
        self.running = True
        
        # Start PostgreSQL NOTIFY stream in a thread
        self.thread = threading.Thread(target=self._stream_loop, daemon=True)
        self.thread.start()
        
        print("🚀 Real-time PostgreSQL trade streaming started")
        
    def stop(self):
        """Stop streaming"""
        self.running = False
        if self.thread:
            self.thread.join(timeout=1)
        if self.conn:
            self.conn.close()
    
    def _connect_db(self):
        """Connect to PostgreSQL with LISTEN"""
        try:
            self.conn = psycopg2.connect(
                host='localhost',
                port=5432,
                database='market_data',
                user='daws'
            )
            self.conn.set_isolation_level(psycopg2.extensions.ISOLATION_LEVEL_AUTOCOMMIT)
            cur = self.conn.cursor()
            cur.execute("LISTEN new_trade;")
            cur.close()
            print("🔗 PostgreSQL connected, listening for new_trade notifications")
        except Exception as e:
            print(f"❌ Failed to connect to PostgreSQL: {e}")
    
    def _stream_loop(self):
        """Main streaming loop using PostgreSQL NOTIFY"""
        print("🚀 Starting PostgreSQL NOTIFY stream loop")
        self._connect_db()
        
        while self.running:
            try:
                if self.conn is None:
                    print("📡 Reconnecting to PostgreSQL...")
                    self._connect_db()
                    time.sleep(1)
                    continue
                
                # Wait for PostgreSQL notifications
                if select.select([self.conn], [], [], 0.1) != ([], [], []):
                    self.conn.poll()
                    notification_count = 0
                    while self.conn.notifies:
                        notify = self.conn.notifies.pop(0)
                        notification_count += 1
                        
                        if notify.payload and self.clients:
                            trade_id = notify.payload
                            print(f"📨 NOTIFY received: trade_id={trade_id}, clients={len(self.clients)}")
                            self._handle_trade_notify(trade_id)
                    
                    if notification_count > 0:
                        print(f"📊 Processed {notification_count} notifications")
                            
            except Exception as e:
                print(f"❌ Error in stream loop: {e}")
                time.sleep(1)
                try:
                    if self.conn:
                        self.conn.close()
                    self._connect_db()
                except:
                    pass
    
    def _handle_trade_notify(self, trade_id):
        """Handle a single trade notification"""
        try:
            cur = self.conn.cursor()
            cur.execute("""
                SELECT trade_id, time, exchange, symbol, price, size, side
                FROM trades
                WHERE trade_id = %s
            """, (trade_id,))
            
            trade = cur.fetchone()
            print(f"🔍 Trade lookup for {trade_id}: {'Found' if trade else 'Not found'}")
            
            if trade:
                trade_data = {
                    'type': 'trade',
                    'trade_id': trade[0],
                    'timestamp': trade[1].isoformat(),
                    'timestamp_ms': int(trade[1].timestamp() * 1000),
                    'exchange': trade[2],
                    'symbol': trade[3],
                    'price': float(trade[4]),
                    'size': float(trade[5]),
                    'side': trade[6] or 'unknown'
                }
                
                print(f"💾 Trade data: {trade[2]} {trade[3]} ${trade[4]}")
                
                # Emit to all connected clients in /realtime namespace with app context
                with self.flask_app.app_context():
                    self.socketio.emit('trade', trade_data, namespace='/realtime')
                    print(f"📡 Streamed: {trade[2]} {trade[3]} ${trade[4]} to {len(self.clients)} clients")
            else:
                print(f"❌ Trade {trade_id} not found in database")
                
            cur.close()
            
        except Exception as e:
            print(f"❌ Error handling trade notify: {e}")
    
    def add_client(self, sid):
        """Add client"""
        self.clients.add(sid)
        logger.info(f"Client connected: {sid} (total: {len(self.clients)})")
    
    def remove_client(self, sid):
        """Remove client"""
        self.clients.discard(sid)
        logger.info(f"Client disconnected: {sid} (remaining: {len(self.clients)})")

# Global streamer instance
streamer = None

def init_socketio(app, socket_io):
    """Initialize SocketIO with the Flask app"""
    global socketio, streamer
    socketio = socket_io
    streamer = TradeStreamer(socketio, app)
    
    @socketio.on('connect', namespace='/realtime')
    def handle_connect():
        """Handle client connection"""
        print(f"Client connected to real-time stream")
        emit('connected', {
            'message': 'Connected to real-time PostgreSQL trade stream',
            'timestamp': datetime.now().isoformat()
        })
        
        # Add client and start streaming if not already running
        from flask import request
        streamer.add_client(request.sid)
        if not streamer.running:
            streamer.start()
        
        # Send a test message to verify events work
        emit('test_message', {'message': 'Test event from Flask-SocketIO'})
        print(f"🧪 Sent test message to client {request.sid}")
        
        # Also send a test trade to see if trade events work in request context
        test_trade = {
            'type': 'trade', 
            'trade_id': 'test_123',
            'timestamp': datetime.now().isoformat(),
            'timestamp_ms': int(datetime.now().timestamp() * 1000),
            'exchange': 'test_exchange',
            'symbol': 'TEST-USD',
            'price': 12345.67,
            'size': 0.001,
            'side': 'buy'
        }
        emit('trade', test_trade)
        print(f"🧪 Sent test trade to client {request.sid}")
    
    @socketio.on('disconnect', namespace='/realtime')
    def handle_disconnect():
        """Handle client disconnection"""
        print(f"Client disconnected from real-time stream")
        from flask import request
        streamer.remove_client(request.sid)
    
    return socketio