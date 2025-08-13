"""
Prometheus metrics endpoint for FastAPI backend

Exposes application metrics in Prometheus format for monitoring.
"""

from fastapi import APIRouter, Response
from prometheus_client import (
    Counter, Histogram, Gauge, Summary,
    generate_latest, CONTENT_TYPE_LATEST,
    CollectorRegistry
)
import time
import psutil
import asyncio
from typing import Optional
import redis.asyncio as redis

router = APIRouter(tags=["metrics"])

# Create metrics
request_count = Counter(
    'http_requests_total',
    'Total HTTP requests',
    ['method', 'endpoint', 'status']
)

request_duration = Histogram(
    'http_request_duration_seconds',
    'HTTP request duration',
    ['method', 'endpoint']
)

active_connections = Gauge(
    'websocket_connections_active',
    'Active WebSocket connections'
)

redis_operations = Counter(
    'redis_operations_total',
    'Total Redis operations',
    ['operation', 'status']
)

stream_messages = Counter(
    'stream_messages_processed_total',
    'Total messages processed from Redis Streams',
    ['stream', 'consumer']
)

system_cpu = Gauge('system_cpu_percent', 'System CPU usage percentage')
system_memory = Gauge('system_memory_percent', 'System memory usage percentage')
system_disk = Gauge('system_disk_percent', 'System disk usage percentage')

# Custom metrics for trading
trades_processed = Counter(
    'trades_processed_total',
    'Total trades processed',
    ['exchange', 'symbol']
)

orderbook_updates = Counter(
    'orderbook_updates_total',
    'Total orderbook updates processed',
    ['exchange', 'symbol']
)

api_latency = Summary(
    'api_latency_seconds',
    'API endpoint latency',
    ['endpoint']
)

# Redis Stream metrics
stream_lag = Gauge(
    'redis_stream_lag',
    'Lag in Redis Stream consumption',
    ['stream', 'consumer_group']
)

stream_length = Gauge(
    'redis_stream_length',
    'Current length of Redis Stream',
    ['stream']
)


class MetricsCollector:
    """Collects and updates system and application metrics"""
    
    def __init__(self, redis_url: str = "redis://localhost:6379"):
        self.redis_url = redis_url
        self.redis_client: Optional[redis.Redis] = None
        
    async def connect_redis(self):
        """Connect to Redis for stream metrics"""
        if not self.redis_client:
            self.redis_client = redis.from_url(
                self.redis_url,
                decode_responses=False
            )
            
    async def collect_system_metrics(self):
        """Collect system-level metrics"""
        # CPU usage
        system_cpu.set(psutil.cpu_percent(interval=1))
        
        # Memory usage
        memory = psutil.virtual_memory()
        system_memory.set(memory.percent)
        
        # Disk usage
        disk = psutil.disk_usage('/')
        system_disk.set(disk.percent)
        
    async def collect_redis_stream_metrics(self):
        """Collect Redis Stream metrics"""
        if not self.redis_client:
            await self.connect_redis()
            
        try:
            # Get all stream keys
            stream_keys = await self.redis_client.keys('trades:*')
            
            for key in stream_keys:
                key_str = key.decode('utf-8') if isinstance(key, bytes) else key
                
                # Get stream length
                length = await self.redis_client.xlen(key)
                stream_length.labels(stream=key_str).set(length)
                
                # Get consumer group info
                try:
                    groups = await self.redis_client.xinfo_groups(key)
                    for group in groups:
                        group_name = group['name'].decode('utf-8') if isinstance(group['name'], bytes) else group['name']
                        lag = group.get('lag', 0)
                        stream_lag.labels(
                            stream=key_str,
                            consumer_group=group_name
                        ).set(lag)
                except:
                    pass  # Group might not exist
                    
        except Exception as e:
            print(f"Error collecting Redis metrics: {e}")
            
    async def collect_all_metrics(self):
        """Collect all metrics"""
        await self.collect_system_metrics()
        await self.collect_redis_stream_metrics()


# Global metrics collector
metrics_collector = MetricsCollector()


@router.get("/metrics", response_class=Response)
async def get_metrics():
    """
    Prometheus metrics endpoint with Redis-based aggregation
    
    Returns metrics in Prometheus text format.
    This endpoint should be scraped by Prometheus.
    """
    # Import here to avoid circular imports
    from metrics_aggregator import metrics_aggregator
    
    try:
        # Ensure aggregator is connected
        if not metrics_aggregator.redis_client:
            await metrics_aggregator.connect()
            
        # Get aggregated metrics from Redis (solves process isolation)
        redis_metrics = await metrics_aggregator.export_prometheus_format()
        
        # Also collect local FastAPI metrics
        await metrics_collector.collect_all_metrics()
        local_metrics = generate_latest().decode('utf-8')
        
        # Combine both sources
        combined_metrics = f"{redis_metrics}\n\n# Local FastAPI Metrics\n{local_metrics}"
        
        return Response(
            content=combined_metrics,
            media_type=CONTENT_TYPE_LATEST
        )
        
    except Exception as e:
        # Fallback to local metrics only
        print(f"Redis metrics aggregation failed: {e}")
        await metrics_collector.collect_all_metrics()
        metrics_data = generate_latest()
        
        return Response(
            content=metrics_data,
            media_type=CONTENT_TYPE_LATEST
        )


@router.get("/metrics/health")
async def metrics_health():
    """Health check for metrics endpoint"""
    return {
        "status": "healthy",
        "metrics_enabled": True,
        "prometheus_endpoint": "/api/metrics"
    }


# Middleware to track request metrics
def track_request(method: str, endpoint: str, status: int, duration: float):
    """Track HTTP request metrics"""
    request_count.labels(
        method=method,
        endpoint=endpoint,
        status=status
    ).inc()
    
    request_duration.labels(
        method=method,
        endpoint=endpoint
    ).observe(duration)
    

def track_trade(exchange: str, symbol: str):
    """Track trade processing"""
    trades_processed.labels(
        exchange=exchange,
        symbol=symbol
    ).inc()
    

def track_orderbook(exchange: str, symbol: str):
    """Track orderbook update"""
    orderbook_updates.labels(
        exchange=exchange,
        symbol=symbol
    ).inc()
    

def track_redis_operation(operation: str, success: bool):
    """Track Redis operation"""
    redis_operations.labels(
        operation=operation,
        status="success" if success else "failure"
    ).inc()
    

def track_stream_message(stream: str, consumer: str):
    """Track Redis Stream message processing"""
    stream_messages.labels(
        stream=stream,
        consumer=consumer
    ).inc()