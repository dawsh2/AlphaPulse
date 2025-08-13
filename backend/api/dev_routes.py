"""
Development-only routes for the dev dashboard

These routes provide unfiltered access to data streams for monitoring
and debugging during the Rust migration. Not for production use.
"""

from fastapi import APIRouter, WebSocket, WebSocketDisconnect, HTTPException
from fastapi.responses import Response
import asyncio
import json
import time
import redis.asyncio as redis
from typing import Dict, Any, Optional
import logging
import psutil
import os

# Import metrics for tracking
try:
    from api.metrics_routes import active_connections as metrics_active_connections
except ImportError:
    metrics_active_connections = None

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/ws/dev", tags=["development"])

class DevFirehoseManager:
    """Manages WebSocket connections for the dev dashboard firehose"""
    
    def __init__(self):
        self.active_connections: list[WebSocket] = []
        self.redis_client: Optional[redis.Redis] = None
        self.running = False
        
    async def connect(self, websocket: WebSocket):
        """Accept a new WebSocket connection"""
        await websocket.accept()
        self.active_connections.append(websocket)
        
        # Update Prometheus metrics
        if metrics_active_connections:
            metrics_active_connections.set(len(self.active_connections))
            
        logger.info(f"Dev dashboard connected. Active connections: {len(self.active_connections)}")
        
    def disconnect(self, websocket: WebSocket):
        """Remove a WebSocket connection"""
        if websocket in self.active_connections:
            self.active_connections.remove(websocket)
            
        # Update Prometheus metrics
        if metrics_active_connections:
            metrics_active_connections.set(len(self.active_connections))
            
        logger.info(f"Dev dashboard disconnected. Active connections: {len(self.active_connections)}")
        
    async def send_to_all(self, message: Dict[str, Any]):
        """Send message to all connected clients"""
        if not self.active_connections:
            return
            
        message_str = json.dumps(message)
        disconnected = []
        
        for connection in self.active_connections:
            try:
                await connection.send_text(message_str)
            except Exception as e:
                logger.error(f"Error sending to client: {e}")
                disconnected.append(connection)
                
        # Clean up disconnected clients
        for conn in disconnected:
            self.disconnect(conn)
            
    async def start_redis_stream(self, redis_url: str = "redis://localhost:6379"):
        """Start consuming from Redis Streams and forwarding to WebSocket clients"""
        if self.running:
            return
            
        self.running = True
        self.redis_client = redis.from_url(redis_url, decode_responses=False)
        
        # Stream patterns to consume ($ means latest data)
        streams = {
            'trades:*': '$',
            'orderbook:*': '$'
        }
        
        while self.running:
            try:
                # Get all stream keys and verify they are actually streams
                all_streams = {}
                for pattern in streams.keys():
                    keys = await self.redis_client.keys(pattern.encode())
                    for key in keys:
                        # Check if key is actually a stream
                        key_type = await self.redis_client.type(key)
                        if key_type == b'stream':
                            all_streams[key] = streams[pattern]
                
                if not all_streams:
                    await asyncio.sleep(1)
                    continue
                    
                # Read from streams with timeout
                try:
                    messages = await self.redis_client.xread(
                        all_streams,
                        block=1000  # 1 second timeout
                    )
                    
                    if messages:
                        # Format and send to dashboard
                        formatted = {
                            "type": "firehose",
                            "timestamp": time.time() * 1000,
                            "streams": {}
                        }
                        
                        for stream_key, stream_messages in messages:
                            stream_name = stream_key.decode() if isinstance(stream_key, bytes) else stream_key
                            formatted["streams"][stream_name] = []
                            
                            for msg_id, fields in stream_messages:
                                formatted_fields = {}
                                for k, v in fields.items():
                                    key = k.decode() if isinstance(k, bytes) else k
                                    val = v.decode() if isinstance(v, bytes) else v
                                    formatted_fields[key] = val
                                    
                                formatted["streams"][stream_name].append({
                                    "id": msg_id.decode() if isinstance(msg_id, bytes) else msg_id,
                                    "fields": formatted_fields
                                })
                                
                        await self.send_to_all(formatted)
                        
                except asyncio.TimeoutError:
                    pass  # Normal timeout, continue
                    
                # Send system status and metrics periodically (every 10 seconds)
                if not hasattr(self, '_last_system_update'):
                    self._last_system_update = 0
                    
                current_time = time.time()
                if current_time - self._last_system_update >= 10:  # Every 10 seconds
                    self._last_system_update = current_time
                    
                    # Send system status
                    import psutil
                    await self.send_to_all({
                        "type": "system",
                        "data": {
                            "cpu_percent": psutil.cpu_percent(interval=None),
                            "memory_percent": psutil.virtual_memory().percent,
                            "disk_percent": psutil.disk_usage('/').percent,
                            "network_rx_kb": 0,  # Simplified for now
                            "network_tx_kb": 0,  # Simplified for now
                            "uptime_seconds": int(time.time() - psutil.boot_time())
                        }
                    })
                    
                    # Send metrics based on stream activity
                    trade_streams = [k for k in all_streams.keys() if b'trades:' in k]
                    orderbook_streams = [k for k in all_streams.keys() if b'orderbook:' in k]
                    
                    await self.send_to_all({
                        "type": "metrics",
                        "data": {
                            "trades_per_second": len(trade_streams) * 10,  # Estimated
                            "orderbook_updates_per_second": len(orderbook_streams) * 5,  # Estimated
                            "total_trades": len(trade_streams) * 1000,  # Estimated
                            "total_orderbook_updates": len(orderbook_streams) * 500,  # Estimated
                            "latency_ms": 2.5,
                            "redis_stream_length": sum([await self.redis_client.xlen(k) for k in all_streams.keys()]),
                            "active_connections": len(self.active_connections)
                        }
                    })
                    
            except Exception as e:
                logger.error(f"Error in Redis stream consumer: {e}")
                await asyncio.sleep(1)
                
    async def stop(self):
        """Stop the Redis stream consumer"""
        self.running = False
        if self.redis_client:
            await self.redis_client.close()
            

# Global manager instance
firehose_manager = DevFirehoseManager()

@router.websocket("/firehose")
async def websocket_firehose(websocket: WebSocket):
    """
    WebSocket endpoint for dev dashboard firehose
    
    Streams all data from Redis Streams without filtering.
    This is for development monitoring only.
    """
    await firehose_manager.connect(websocket)
    
    # Start Redis stream consumer if not running
    if not firehose_manager.running:
        asyncio.create_task(firehose_manager.start_redis_stream())
    
    try:
        # Keep connection alive and handle incoming messages
        while True:
            # Wait for any message from client (ping/pong or commands)
            data = await websocket.receive_text()
            
            # Handle client messages if needed
            if data:
                try:
                    message = json.loads(data)
                    
                    # Handle subscription requests
                    if message.get("type") == "subscribe":
                        # Send confirmation
                        await websocket.send_json({
                            "type": "subscribed",
                            "streams": message.get("streams", [])
                        })
                        
                except json.JSONDecodeError:
                    pass  # Ignore invalid JSON
                    
    except WebSocketDisconnect:
        firehose_manager.disconnect(websocket)
    except Exception as e:
        logger.error(f"WebSocket error: {e}")
        firehose_manager.disconnect(websocket)
        

# Also create a simple test endpoint that doesn't require Redis
@router.websocket("/test")
async def websocket_test(websocket: WebSocket):
    """Test WebSocket endpoint with simulated data"""
    await websocket.accept()
    
    try:
        while True:
            # Send simulated trade data (rotate between exchanges)
            import random
            exchanges = ["coinbase", "kraken", "binance"]
            
            # Send multiple trades per update for more activity
            for _ in range(random.randint(1, 3)):
                exchange = random.choice(exchanges)
                base_price = 50000
                price_variation = random.uniform(-500, 500)
                
                await websocket.send_json({
                    "type": "trade",
                    "data": {
                        "timestamp": int(time.time() * 1000) + random.randint(0, 100),
                        "symbol": "BTC-USD",
                        "exchange": exchange,
                        "price": base_price + price_variation,
                        "volume": random.uniform(0.001, 0.5),
                        "side": random.choice(["buy", "sell"]),
                        "trade_id": str(int(time.time() * 1000000) + random.randint(0, 1000))
                    }
                })
            
            # Send simulated orderbook
            await websocket.send_json({
                "type": "orderbook",
                "data": {
                    "symbol": "BTC-USD",
                    "exchange": exchange,
                    "timestamp": int(time.time() * 1000),
                    "bids": [
                        {"price": 49900 - i * 10, "size": 0.1 * (i + 1)}
                        for i in range(20)
                    ],
                    "asks": [
                        {"price": 50100 + i * 10, "size": 0.1 * (i + 1)}
                        for i in range(20)
                    ]
                }
            })
            
            # Send metrics
            await websocket.send_json({
                "type": "metrics",
                "data": {
                    "trades_per_second": 100 + (time.time() % 50),
                    "orderbook_updates_per_second": 50 + (time.time() % 25),
                    "latency_ms": 1 + (time.time() % 5),
                    "redis_stream_length": int(time.time() % 10000),
                    "active_connections": len(firehose_manager.active_connections)
                }
            })
            
            # Send real system status
            process = psutil.Process(os.getpid())
            
            # Get network I/O stats
            net_io = psutil.net_io_counters()
            if not hasattr(websocket, '_last_net_io'):
                websocket._last_net_io = (net_io.bytes_recv, net_io.bytes_sent, time.time())
                network_rx_kb = 0
                network_tx_kb = 0
            else:
                last_recv, last_sent, last_time = websocket._last_net_io
                time_delta = time.time() - last_time
                network_rx_kb = (net_io.bytes_recv - last_recv) / 1024 / max(time_delta, 0.1)
                network_tx_kb = (net_io.bytes_sent - last_sent) / 1024 / max(time_delta, 0.1)
                websocket._last_net_io = (net_io.bytes_recv, net_io.bytes_sent, time.time())
            
            await websocket.send_json({
                "type": "system",
                "data": {
                    "cpu_percent": psutil.cpu_percent(interval=None),
                    "memory_percent": psutil.virtual_memory().percent,
                    "disk_percent": psutil.disk_usage('/').percent,
                    "network_rx_kb": round(network_rx_kb, 1),
                    "network_tx_kb": round(network_tx_kb, 1),
                    "uptime_seconds": int(time.time() - psutil.boot_time())
                }
            })
            
            await asyncio.sleep(0.5)  # Send updates every 500ms
            
    except WebSocketDisconnect:
        pass
    except Exception as e:
        logger.error(f"Test WebSocket error: {e}")