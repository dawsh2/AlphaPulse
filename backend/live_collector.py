#!/usr/bin/env python3
"""
Simple live Coinbase WebSocket collector for dashboard testing
Writes trades directly to Redis Streams
"""

import asyncio
import json
import redis.asyncio as redis
import websockets
import time
from datetime import datetime
import logging

# Import metrics for tracking
try:
    from api.metrics_routes import trades_processed, orderbook_updates
except ImportError:
    trades_processed = None
    orderbook_updates = None

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class CoinbaseLiveCollector:
    def __init__(self):
        self.redis_client = None
        
    async def connect_redis(self):
        """Connect to Redis"""
        self.redis_client = redis.from_url("redis://localhost:6379")
        logger.info("Connected to Redis")
        
    async def write_trade(self, trade_data):
        """Write trade to Redis Stream"""
        if not self.redis_client:
            return
            
        stream_key = f"trades:{trade_data['exchange']}:{trade_data['symbol']}"
        
        try:
            # Use XADD for Redis Streams (not SET)
            await self.redis_client.xadd(stream_key, trade_data)
            
            # Update Prometheus metrics
            if trades_processed:
                trades_processed.labels(
                    exchange=trade_data['exchange'],
                    symbol=trade_data['symbol']
                ).inc()
                
            logger.info(f"Written trade: {trade_data['symbol']} @ ${trade_data['price']}")
        except Exception as e:
            logger.error(f"Redis write error: {e}")
    
    async def write_orderbook(self, orderbook_data):
        """Write orderbook to Redis Stream"""
        if not self.redis_client:
            return
            
        stream_key = f"orderbook:{orderbook_data['exchange']}:{orderbook_data['symbol']}"
        
        try:
            # Use XADD for Redis Streams
            await self.redis_client.xadd(stream_key, orderbook_data)
            
            # Update Prometheus metrics
            if orderbook_updates:
                orderbook_updates.labels(
                    exchange=orderbook_data['exchange'],
                    symbol=orderbook_data['symbol']
                ).inc()
                
            logger.info(f"Written orderbook: {orderbook_data['symbol']} spread=${orderbook_data.get('spread', 'N/A')}")
        except Exception as e:
            logger.error(f"Redis orderbook write error: {e}")
    
    async def collect_coinbase(self):
        """Collect live trades from Coinbase WebSocket"""
        uri = "wss://ws-feed.exchange.coinbase.com"
        
        # Subscribe to BTC-USD trades AND orderbook (using ticker for spread info)
        subscribe_message = {
            "type": "subscribe",
            "product_ids": ["BTC-USD", "ETH-USD"],
            "channels": ["matches", "ticker"]
        }
        
        try:
            async with websockets.connect(uri) as websocket:
                logger.info("Connected to Coinbase WebSocket")
                
                # Send subscription
                await websocket.send(json.dumps(subscribe_message))
                logger.info("Subscribed to BTC-USD and ETH-USD trades")
                
                # Listen for messages
                async for message in websocket:
                    try:
                        data = json.loads(message)
                        
                        # Debug: log message types we receive
                        msg_type = data.get("type")
                        if msg_type not in ["match", "heartbeat"]:
                            logger.info(f"Received message type: {msg_type}")
                        
                        if data.get("type") == "match":
                            # Convert to our format
                            trade = {
                                "timestamp": str(int(time.time())),
                                "symbol": data["product_id"].replace("-", "-"),
                                "exchange": "coinbase",
                                "price": str(float(data["price"])),
                                "volume": str(float(data["size"])), 
                                "side": data["side"],
                                "trade_id": data["trade_id"],
                                "ingested_at": str(int(time.time() * 1000))
                            }
                            
                            await self.write_trade(trade)
                            
                        elif data.get("type") == "ticker":
                            # Ticker update with spread info
                            symbol = data["product_id"]
                            best_bid = float(data.get("best_bid", 0))
                            best_ask = float(data.get("best_ask", 0))
                            
                            if best_bid > 0 and best_ask > 0:
                                spread = best_ask - best_bid
                                
                                # Create simplified orderbook from ticker
                                bids = [{"price": best_bid, "size": 1.0}]  # Mock size
                                asks = [{"price": best_ask, "size": 1.0}]  # Mock size
                                
                                orderbook = {
                                    "timestamp": str(int(time.time())),
                                    "symbol": symbol.replace("-", "-"),
                                    "exchange": "coinbase",
                                    "bids": json.dumps(bids),
                                    "asks": json.dumps(asks), 
                                    "spread": str(round(spread, 2)),
                                    "best_bid": str(best_bid),
                                    "best_ask": str(best_ask),
                                    "ingested_at": str(int(time.time() * 1000))
                                }
                                
                                await self.write_orderbook(orderbook)
                            
                    except json.JSONDecodeError:
                        logger.warning("Invalid JSON received")
                    except Exception as e:
                        logger.error(f"Error processing message: {e}")
                        
        except Exception as e:
            logger.error(f"WebSocket connection error: {e}")
            await asyncio.sleep(5)  # Wait before reconnecting
            
    async def run(self):
        """Main run loop with auto-reconnect"""
        await self.connect_redis()
        
        while True:
            try:
                logger.info("Starting Coinbase collector...")
                await self.collect_coinbase()
            except Exception as e:
                logger.error(f"Collector error: {e}")
                logger.info("Reconnecting in 5 seconds...")
                await asyncio.sleep(5)
            finally:
                # Clean disconnect
                if self.redis_client:
                    await self.redis_client.close()
                    self.redis_client = None
                await self.connect_redis()

if __name__ == "__main__":
    collector = CoinbaseLiveCollector()
    asyncio.run(collector.run())