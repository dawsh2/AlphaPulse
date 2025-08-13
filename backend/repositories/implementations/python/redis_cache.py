"""
Redis implementation of CacheRepository
Provides distributed caching with TTL support
"""
from typing import Optional, Any
import json
import pickle
import redis
import logging
from datetime import timedelta

logger = logging.getLogger(__name__)


class RedisCacheRepository:
    """
    Redis implementation of CacheRepository protocol
    Provides high-performance distributed caching
    """
    
    def __init__(
        self,
        host: str = 'localhost',
        port: int = 6379,
        db: int = 0,
        password: Optional[str] = None,
        decode_responses: bool = False,
        connection_pool: Optional[redis.ConnectionPool] = None
    ):
        """Initialize Redis connection"""
        try:
            if connection_pool:
                self.redis = redis.Redis(connection_pool=connection_pool)
            else:
                self.redis = redis.Redis(
                    host=host,
                    port=port,
                    db=db,
                    password=password,
                    decode_responses=decode_responses
                )
            
            # Test connection
            self.redis.ping()
            logger.info(f"RedisCacheRepository connected to Redis at {host}:{port}")
        except redis.ConnectionError as e:
            logger.warning(f"Redis connection failed: {e}. Using mock cache.")
            self.redis = None
            self._memory_cache = {}  # Fallback to memory cache
    
    async def get(self, key: str) -> Optional[Any]:
        """Get value from cache"""
        try:
            if not self.redis:
                return self._memory_cache.get(key)
            
            value = self.redis.get(key)
            if value is None:
                return None
            
            # Try to deserialize as JSON first, then pickle
            try:
                return json.loads(value)
            except (json.JSONDecodeError, TypeError):
                try:
                    return pickle.loads(value)
                except:
                    # Return as string if can't deserialize
                    return value.decode('utf-8') if isinstance(value, bytes) else value
        except Exception as e:
            logger.error(f"Cache get error for key {key}: {e}")
            return None
    
    async def set(
        self,
        key: str,
        value: Any,
        ttl: Optional[int] = None
    ) -> bool:
        """Set value in cache with optional TTL (in seconds)"""
        try:
            if not self.redis:
                self._memory_cache[key] = value
                return True
            
            # Serialize value
            try:
                serialized = json.dumps(value)
            except (TypeError, ValueError):
                # Fall back to pickle for complex objects
                serialized = pickle.dumps(value)
            
            if ttl:
                return bool(self.redis.setex(key, ttl, serialized))
            else:
                return bool(self.redis.set(key, serialized))
        except Exception as e:
            logger.error(f"Cache set error for key {key}: {e}")
            return False
    
    async def delete(self, key: str) -> bool:
        """Delete key from cache"""
        try:
            if not self.redis:
                if key in self._memory_cache:
                    del self._memory_cache[key]
                    return True
                return False
            
            return bool(self.redis.delete(key))
        except Exception as e:
            logger.error(f"Cache delete error for key {key}: {e}")
            return False
    
    async def exists(self, key: str) -> bool:
        """Check if key exists in cache"""
        try:
            if not self.redis:
                return key in self._memory_cache
            
            return bool(self.redis.exists(key))
        except Exception as e:
            logger.error(f"Cache exists error for key {key}: {e}")
            return False
    
    async def clear_pattern(self, pattern: str) -> int:
        """Clear all keys matching pattern"""
        try:
            if not self.redis:
                # Simple pattern matching for memory cache
                keys_to_delete = [k for k in self._memory_cache.keys() if pattern.replace('*', '') in k]
                for key in keys_to_delete:
                    del self._memory_cache[key]
                return len(keys_to_delete)
            
            # Use SCAN to avoid blocking on large datasets
            deleted_count = 0
            cursor = 0
            
            while True:
                cursor, keys = self.redis.scan(cursor, match=pattern, count=100)
                if keys:
                    deleted_count += self.redis.delete(*keys)
                if cursor == 0:
                    break
            
            return deleted_count
        except Exception as e:
            logger.error(f"Cache clear pattern error for pattern {pattern}: {e}")
            return 0
    
    # Additional utility methods
    
    async def increment(self, key: str, amount: int = 1) -> int:
        """Increment a counter"""
        try:
            if not self.redis:
                current = self._memory_cache.get(key, 0)
                self._memory_cache[key] = current + amount
                return self._memory_cache[key]
            
            return self.redis.incrby(key, amount)
        except Exception as e:
            logger.error(f"Cache increment error for key {key}: {e}")
            return 0
    
    async def get_many(self, keys: list) -> dict:
        """Get multiple values at once"""
        try:
            if not self.redis:
                return {k: self._memory_cache.get(k) for k in keys}
            
            values = self.redis.mget(keys)
            result = {}
            
            for key, value in zip(keys, values):
                if value is not None:
                    try:
                        result[key] = json.loads(value)
                    except:
                        try:
                            result[key] = pickle.loads(value)
                        except:
                            result[key] = value.decode('utf-8') if isinstance(value, bytes) else value
            
            return result
        except Exception as e:
            logger.error(f"Cache get_many error: {e}")
            return {}
    
    async def set_many(self, mapping: dict, ttl: Optional[int] = None) -> bool:
        """Set multiple values at once"""
        try:
            if not self.redis:
                self._memory_cache.update(mapping)
                return True
            
            # Serialize all values
            serialized = {}
            for key, value in mapping.items():
                try:
                    serialized[key] = json.dumps(value)
                except:
                    serialized[key] = pickle.dumps(value)
            
            # Use pipeline for atomic operation
            pipe = self.redis.pipeline()
            
            for key, value in serialized.items():
                if ttl:
                    pipe.setex(key, ttl, value)
                else:
                    pipe.set(key, value)
            
            results = pipe.execute()
            return all(results)
        except Exception as e:
            logger.error(f"Cache set_many error: {e}")
            return False