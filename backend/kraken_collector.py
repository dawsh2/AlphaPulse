#!/usr/bin/env python3
"""
Kraken WebSocket collector for dashboard testing
Writes trades and orderbooks directly to Redis Streams
"""

import asyncio
import json
import redis.asyncio as redis
import websockets
import time
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class KrakenLiveCollector:
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
            await self.redis_client.xadd(stream_key, trade_data)
            logger.info(f"Written trade: {trade_data['symbol']} @ ${trade_data['price']}")
        except Exception as e:
            logger.error(f"Redis write error: {e}")
    
    async def write_orderbook(self, orderbook_data):
        """Write orderbook to Redis Stream"""
        if not self.redis_client:
            return
            
        stream_key = f"orderbook:{orderbook_data['exchange']}:{orderbook_data['symbol']}"
        
        try:
            await self.redis_client.xadd(stream_key, orderbook_data)
            logger.info(f"Written orderbook: {orderbook_data['symbol']} spread=${orderbook_data.get('spread', 'N/A')}")
        except Exception as e:
            logger.error(f"Redis orderbook write error: {e}")
    
    async def collect_kraken(self):
        """Collect live trades from Kraken WebSocket"""
        uri = "wss://ws.kraken.com"
        
        # Subscribe to BTC/USD and ETH/USD trades and ticker
        subscribe_message = {
            "event": "subscribe",
            "pair": ["XBT/USD", "ETH/USD"],
            "subscription": {"name": "trade"}
        }
        
        ticker_subscribe = {
            "event": "subscribe", 
            "pair": ["XBT/USD", "ETH/USD"],
            "subscription": {"name": "ticker"}
        }
        
        try:
            async with websockets.connect(uri) as websocket:
                logger.info("Connected to Kraken WebSocket")
                
                # Send subscriptions
                await websocket.send(json.dumps(subscribe_message))
                await websocket.send(json.dumps(ticker_subscribe))
                logger.info("Subscribed to BTC/USD and ETH/USD trades and tickers")
                
                async for message in websocket:
                    try:
                        data = json.loads(message)
                        
                        # Skip system messages
                        if isinstance(data, dict):
                            if data.get("event") in ["subscriptionStatus", "systemStatus", "heartbeat"]:
                                continue
                        
                        # Handle trade messages
                        if isinstance(data, list) and len(data) >= 4:
                            channel_id = data[0] if isinstance(data[0], int) else None
                            trades_data = data[1] if len(data) > 1 and isinstance(data[1], list) else None
                            channel_name = data[2] if len(data) > 2 else ""
                            pair = data[3] if len(data) > 3 else ""
                            
                            # Process trades
                            if trades_data and channel_name == "trade":
                                for trade in trades_data:
                                    if len(trade) >= 6:
                                        price = float(trade[0])
                                        volume = float(trade[1])
                                        timestamp = float(trade[2])
                                        side = "buy" if trade[3] == "b" else "sell"
                                        
                                        # Convert pair format: XBT/USD -> BTC-USD
                                        symbol = pair.replace("XBT", "BTC").replace("/", "-")
                                        
                                        trade_record = {
                                            "timestamp": str(int(timestamp)),
                                            "symbol": symbol,
                                            "exchange": "kraken",
                                            "price": str(price),
                                            "volume": str(volume),
                                            "side": side,
                                            "trade_id": str(int(timestamp * 1000000)),
                                            "ingested_at": str(int(time.time() * 1000))
                                        }
                                        
                                        await self.write_trade(trade_record)
                            
                            # Process ticker (for orderbook/spread info)
                            elif trades_data and channel_name == "ticker" and isinstance(trades_data, dict):
                                ticker = trades_data
                                
                                if "b" in ticker and "a" in ticker:  # best bid/ask
                                    best_bid = float(ticker["b"][0])
                                    best_ask = float(ticker["a"][0])
                                    spread = best_ask - best_bid
                                    
                                    # Convert pair format
                                    symbol = pair.replace("XBT", "BTC").replace("/", "-")
                                    
                                    # Mock orderbook from ticker
                                    bids = [{"price": best_bid, "size": 1.0}]
                                    asks = [{"price": best_ask, "size": 1.0}]
                                    
                                    orderbook = {
                                        "timestamp": str(int(time.time())),
                                        "symbol": symbol,
                                        "exchange": "kraken",
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
            await asyncio.sleep(5)
            
    async def run(self):
        """Main run loop with auto-reconnect"""
        await self.connect_redis()
        
        while True:
            try:
                logger.info("Starting Kraken collector...")
                await self.collect_kraken()
            except Exception as e:
                logger.error(f"Collector error: {e}")
                logger.info("Reconnecting in 5 seconds...")
                await asyncio.sleep(5)

if __name__ == "__main__":
    collector = KrakenLiveCollector()
    asyncio.run(collector.run())