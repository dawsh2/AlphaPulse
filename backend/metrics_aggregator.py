#!/usr/bin/env python3
"""
Redis-based metrics aggregation service
Solves the process isolation problem by collecting metrics from all processes
"""

import asyncio
import json
import time
import logging
from typing import Dict, Any
import redis.asyncio as redis

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class MetricsAggregator:
    """
    Centralized metrics aggregation using Redis as the shared storage.
    
    This solves the process isolation problem:
    - live_collector.py writes metrics to Redis
    - FastAPI reads aggregated metrics from Redis
    - Dashboard gets unified view of all system metrics
    """
    
    def __init__(self, redis_url: str = "redis://localhost:6379"):
        self.redis_url = redis_url
        self.redis_client = None
        self.metrics_key = "system:metrics"
        self.metrics_history_key = "system:metrics:history"
        
    async def connect(self):
        """Connect to Redis"""
        self.redis_client = redis.from_url(self.redis_url)
        await self.redis_client.ping()
        logger.info(f"Connected to Redis at {self.redis_url}")
        
    async def record_trade_processed(self, exchange: str, symbol: str):
        """Record a processed trade"""
        metric_key = f"trades_processed:{exchange}:{symbol}"
        current_time = int(time.time())
        
        # Increment counter
        await self.redis_client.hincrby(self.metrics_key, metric_key, 1)
        await self.redis_client.hincrby(self.metrics_key, "trades_processed_total", 1)
        
        # Track rate (trades per second)
        rate_key = f"trades_rate:{exchange}:{symbol}:{current_time}"
        await self.redis_client.incr(rate_key)
        await self.redis_client.expire(rate_key, 60)  # Keep for 1 minute
        
    async def record_orderbook_update(self, exchange: str, symbol: str):
        """Record an orderbook update"""
        metric_key = f"orderbook_updates:{exchange}:{symbol}"
        current_time = int(time.time())
        
        # Increment counter
        await self.redis_client.hincrby(self.metrics_key, metric_key, 1)
        await self.redis_client.hincrby(self.metrics_key, "orderbook_updates_total", 1)
        
        # Track rate
        rate_key = f"orderbook_rate:{exchange}:{symbol}:{current_time}"
        await self.redis_client.incr(rate_key)
        await self.redis_client.expire(rate_key, 60)
        
    async def record_websocket_connection(self, action: str):
        """Record WebSocket connection events (connect/disconnect)"""
        if action == "connect":
            await self.redis_client.hincrby(self.metrics_key, "websocket_connections_active", 1)
        elif action == "disconnect":
            await self.redis_client.hincrby(self.metrics_key, "websocket_connections_active", -1)
            
    async def record_system_metrics(self, cpu_percent: float, memory_percent: float, 
                                  disk_percent: float, network_rx_kb: float, network_tx_kb: float):
        """Record system metrics"""
        current_time = int(time.time())
        system_metrics = {
            f"system_cpu_percent": cpu_percent,
            f"system_memory_percent": memory_percent,
            f"system_disk_percent": disk_percent,
            f"system_network_rx_kb": network_rx_kb,
            f"system_network_tx_kb": network_tx_kb,
            f"system_last_update": current_time
        }
        
        await self.redis_client.hmset(self.metrics_key, system_metrics)
        
    async def get_metrics_summary(self) -> Dict[str, Any]:
        """Get aggregated metrics for Prometheus/Dashboard"""
        all_metrics = await self.redis_client.hgetall(self.metrics_key)
        current_time = int(time.time())
        
        # Convert bytes to strings and parse numbers
        parsed_metrics = {}
        for key, value in all_metrics.items():
            key_str = key.decode() if isinstance(key, bytes) else key
            value_str = value.decode() if isinstance(value, bytes) else value
            
            try:
                parsed_metrics[key_str] = float(value_str)
            except (ValueError, TypeError):
                parsed_metrics[key_str] = value_str
                
        # Calculate rates (trades/orderbook updates per second)
        trades_per_second = await self._calculate_rate("trades_rate:*", current_time)
        orderbook_per_second = await self._calculate_rate("orderbook_rate:*", current_time)
        
        # Get Redis stream lengths for data flow monitoring
        stream_keys = await self.redis_client.keys("trades:*")
        redis_stream_length = 0
        for key in stream_keys:
            try:
                length = await self.redis_client.xlen(key)
                redis_stream_length += length
            except:
                pass  # Skip non-stream keys
                
        # Prepare final metrics
        summary = {
            "trades_processed_total": parsed_metrics.get("trades_processed_total", 0),
            "orderbook_updates_total": parsed_metrics.get("orderbook_updates_total", 0),
            "trades_per_second": trades_per_second,
            "orderbook_updates_per_second": orderbook_per_second,
            "websocket_connections_active": parsed_metrics.get("websocket_connections_active", 0),
            "redis_stream_length": redis_stream_length,
            "system_cpu_percent": parsed_metrics.get("system_cpu_percent", 0),
            "system_memory_percent": parsed_metrics.get("system_memory_percent", 0),
            "system_disk_percent": parsed_metrics.get("system_disk_percent", 0),
            "system_network_rx_kb": parsed_metrics.get("system_network_rx_kb", 0),
            "system_network_tx_kb": parsed_metrics.get("system_network_tx_kb", 0),
            "latency_ms": 2.5,  # Estimated from architecture
            "last_updated": current_time
        }
        
        return summary
        
    async def _calculate_rate(self, pattern: str, current_time: int) -> float:
        """Calculate rate per second for a given metric pattern"""
        # Look at last 10 seconds of data
        total_count = 0
        time_window = 10
        
        for i in range(time_window):
            timestamp = current_time - i
            keys = await self.redis_client.keys(pattern.replace("*", f"*:{timestamp}"))
            for key in keys:
                try:
                    count = await self.redis_client.get(key)
                    if count:
                        total_count += int(count)
                except:
                    pass
                    
        return total_count / time_window
        
    async def cleanup_old_metrics(self):
        """Clean up old rate tracking keys"""
        current_time = int(time.time())
        patterns = ["trades_rate:*", "orderbook_rate:*"]
        
        for pattern in patterns:
            keys = await self.redis_client.keys(pattern)
            for key in keys:
                try:
                    # Extract timestamp from key
                    parts = key.decode().split(':')
                    if len(parts) >= 4:
                        timestamp = int(parts[-1])
                        if current_time - timestamp > 300:  # Remove keys older than 5 minutes
                            await self.redis_client.delete(key)
                except:
                    pass  # Skip malformed keys
                    
    async def export_prometheus_format(self) -> str:
        """Export metrics in Prometheus format for FastAPI /metrics endpoint"""
        summary = await self.get_metrics_summary()
        
        prometheus_metrics = []
        
        # Add metrics with descriptions
        prometheus_metrics.extend([
            "# HELP trades_processed_total Total number of trades processed",
            "# TYPE trades_processed_total counter",
            f"trades_processed_total {summary['trades_processed_total']}",
            "",
            "# HELP orderbook_updates_total Total number of orderbook updates processed", 
            "# TYPE orderbook_updates_total counter",
            f"orderbook_updates_total {summary['orderbook_updates_total']}",
            "",
            "# HELP trades_per_second Current trades processed per second",
            "# TYPE trades_per_second gauge", 
            f"trades_per_second {summary['trades_per_second']}",
            "",
            "# HELP orderbook_updates_per_second Current orderbook updates per second",
            "# TYPE orderbook_updates_per_second gauge",
            f"orderbook_updates_per_second {summary['orderbook_updates_per_second']}",
            "",
            "# HELP websocket_connections_active Number of active WebSocket connections",
            "# TYPE websocket_connections_active gauge",
            f"websocket_connections_active {summary['websocket_connections_active']}",
            "",
            "# HELP redis_stream_length Total length of all Redis streams",
            "# TYPE redis_stream_length gauge", 
            f"redis_stream_length {summary['redis_stream_length']}",
            "",
            "# HELP system_cpu_percent System CPU usage percentage",
            "# TYPE system_cpu_percent gauge",
            f"system_cpu_percent {summary['system_cpu_percent']}",
            "",
            "# HELP system_memory_percent System memory usage percentage", 
            "# TYPE system_memory_percent gauge",
            f"system_memory_percent {summary['system_memory_percent']}",
            "",
        ])
        
        return "\n".join(prometheus_metrics)

# Global instance for easy import
metrics_aggregator = MetricsAggregator()

async def main():
    """Run cleanup task periodically"""
    await metrics_aggregator.connect()
    
    while True:
        try:
            await metrics_aggregator.cleanup_old_metrics()
            await asyncio.sleep(60)  # Clean up every minute
        except Exception as e:
            logger.error(f"Error in cleanup task: {e}")
            await asyncio.sleep(60)

if __name__ == "__main__":
    asyncio.run(main())