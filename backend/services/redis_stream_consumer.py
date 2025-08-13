"""
Redis Stream Consumer Service

This service consumes trade data from Redis Streams written by Rust collectors.
It demonstrates the multi-consumer pattern for the migration architecture.
"""

import asyncio
import json
import logging
from typing import Dict, List, Optional, Any
from datetime import datetime
import redis.asyncio as redis
from dataclasses import dataclass

logger = logging.getLogger(__name__)

@dataclass
class Trade:
    """Trade data from Redis Stream"""
    timestamp: int
    price: float
    volume: float
    side: str
    trade_id: str
    symbol: str
    exchange: str
    ingested_at: int
    
    @classmethod
    def from_redis_fields(cls, fields: Dict[bytes, bytes]) -> 'Trade':
        """Create Trade from Redis Stream fields"""
        return cls(
            timestamp=int(fields[b'timestamp']),
            price=float(fields[b'price']),
            volume=float(fields[b'volume']),
            side=fields[b'side'].decode('utf-8'),
            trade_id=fields[b'trade_id'].decode('utf-8'),
            symbol=fields[b'symbol'].decode('utf-8'),
            exchange=fields[b'exchange'].decode('utf-8'),
            ingested_at=int(fields[b'ingested_at'])
        )


class RedisStreamConsumer:
    """
    Consumer for Redis Streams written by Rust collectors.
    
    This implements the consumer side of the Redis Streams pattern,
    allowing multiple consumers to read from the same stream.
    """
    
    def __init__(
        self,
        redis_url: str = "redis://localhost:6379",
        consumer_group: str = "python-analytics",
        consumer_name: str = "analytics-1"
    ):
        self.redis_url = redis_url
        self.consumer_group = consumer_group
        self.consumer_name = consumer_name
        self.redis_client: Optional[redis.Redis] = None
        self.running = False
        
    async def connect(self):
        """Connect to Redis"""
        self.redis_client = redis.from_url(
            self.redis_url,
            decode_responses=False  # We'll decode manually
        )
        await self.redis_client.ping()
        logger.info(f"Connected to Redis at {self.redis_url}")
        
    async def create_consumer_groups(self, stream_patterns: List[str]):
        """Create consumer groups for streams if they don't exist"""
        if not self.redis_client:
            await self.connect()
            
        # Find all matching streams
        all_streams = []
        for pattern in stream_patterns:
            keys = await self.redis_client.keys(pattern)
            all_streams.extend(keys)
            
        # Create consumer group for each stream
        for stream_key in all_streams:
            try:
                # Try to create consumer group from beginning of stream
                await self.redis_client.xgroup_create(
                    stream_key,
                    self.consumer_group,
                    id='0'  # Start from beginning
                )
                logger.info(f"Created consumer group {self.consumer_group} for {stream_key}")
            except redis.ResponseError as e:
                if "BUSYGROUP" in str(e):
                    # Group already exists
                    logger.debug(f"Consumer group {self.consumer_group} already exists for {stream_key}")
                else:
                    raise
                    
    async def consume_trades(
        self,
        stream_patterns: List[str] = None,
        block_ms: int = 1000,
        count: int = 10,
        callback=None
    ):
        """
        Consume trades from Redis Streams
        
        Args:
            stream_patterns: List of stream key patterns to consume (e.g., ["trades:*"])
            block_ms: How long to block waiting for data
            count: Max number of messages to read per stream
            callback: Async function to call with each trade
        """
        if not stream_patterns:
            stream_patterns = ["trades:*"]
            
        if not self.redis_client:
            await self.connect()
            
        # Create consumer groups
        await self.create_consumer_groups(stream_patterns)
        
        self.running = True
        logger.info(f"Starting consumption from streams: {stream_patterns}")
        
        # Find all matching streams
        all_streams = []
        for pattern in stream_patterns:
            keys = await self.redis_client.keys(pattern)
            all_streams.extend(keys)
            
        if not all_streams:
            logger.warning(f"No streams found matching patterns: {stream_patterns}")
            return
            
        # Build streams dict for XREADGROUP
        streams = {stream: '>' for stream in all_streams}  # '>' means new messages only
        
        while self.running:
            try:
                # Read from multiple streams
                messages = await self.redis_client.xreadgroup(
                    self.consumer_group,
                    self.consumer_name,
                    streams,
                    count=count,
                    block=block_ms
                )
                
                if messages:
                    for stream_key, stream_messages in messages:
                        logger.debug(f"Received {len(stream_messages)} messages from {stream_key}")
                        
                        for message_id, fields in stream_messages:
                            try:
                                # Parse trade from Redis fields
                                trade = Trade.from_redis_fields(fields)
                                
                                # Process trade (call callback or handle internally)
                                if callback:
                                    await callback(trade)
                                else:
                                    await self.process_trade(trade)
                                    
                                # Acknowledge message
                                await self.redis_client.xack(
                                    stream_key,
                                    self.consumer_group,
                                    message_id
                                )
                                
                            except Exception as e:
                                logger.error(f"Error processing message {message_id}: {e}")
                                # Don't ACK on error - message will be retried
                                
            except Exception as e:
                logger.error(f"Error consuming from streams: {e}")
                await asyncio.sleep(1)  # Brief pause before retry
                
    async def process_trade(self, trade: Trade):
        """
        Default trade processing - override or provide callback
        """
        logger.info(
            f"Trade: {trade.exchange} {trade.symbol} "
            f"${trade.price:.2f} x {trade.volume:.4f} ({trade.side})"
        )
        
    async def get_stream_info(self, stream_pattern: str = "trades:*") -> Dict[str, Any]:
        """Get information about Redis Streams"""
        if not self.redis_client:
            await self.connect()
            
        info = {}
        keys = await self.redis_client.keys(stream_pattern)
        
        for key in keys:
            key_str = key.decode('utf-8') if isinstance(key, bytes) else key
            
            # Get stream info
            length = await self.redis_client.xlen(key)
            
            # Get consumer group info
            groups = await self.redis_client.xinfo_groups(key)
            
            # Get last entry
            last_entry = await self.redis_client.xrevrange(key, count=1)
            
            info[key_str] = {
                'length': length,
                'groups': [
                    {
                        'name': g['name'].decode('utf-8') if isinstance(g['name'], bytes) else g['name'],
                        'consumers': g['consumers'],
                        'pending': g['pending']
                    }
                    for g in groups
                ],
                'last_entry': last_entry[0] if last_entry else None
            }
            
        return info
        
    async def stop(self):
        """Stop consuming"""
        self.running = False
        if self.redis_client:
            await self.redis_client.close()
            

# Example usage and testing
async def example_callback(trade: Trade):
    """Example callback for processing trades"""
    print(f"[{trade.exchange}] {trade.symbol}: ${trade.price:.2f} x {trade.volume}")
    

async def main():
    """Test the Redis Stream consumer"""
    consumer = RedisStreamConsumer(
        consumer_group="test-group",
        consumer_name="test-consumer-1"
    )
    
    # Get stream info
    info = await consumer.get_stream_info()
    print("Stream Info:", json.dumps(info, indent=2, default=str))
    
    # Start consuming
    try:
        await consumer.consume_trades(
            stream_patterns=["trades:*"],
            callback=example_callback
        )
    except KeyboardInterrupt:
        print("\nStopping consumer...")
        await consumer.stop()
        

if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())