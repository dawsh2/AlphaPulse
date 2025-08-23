#!/usr/bin/env python3
"""
Pipeline Interceptor

Provides hooks and monitoring capabilities for each stage of the data pipeline:
Polygon API → Collector → Binary Protocol → Relay → WebSocket → Frontend

This enables complete message tracing and transformation recording for
deep equality validation.
"""

import asyncio
import json
import socket
import struct
import time
import threading
import websockets
from typing import Dict, List, Any, Optional, Callable, Set
from dataclasses import dataclass
from queue import Queue, Empty
import logging
from .deep_equality_framework import DeepEqualityFramework

logger = logging.getLogger(__name__)


@dataclass
class InterceptedMessage:
    """Represents a message intercepted at a pipeline stage"""
    message_id: str
    stage_name: str
    timestamp: float
    data: Any
    metadata: Dict[str, Any]


class PolygonAPIInterceptor:
    """
    Intercepts raw Polygon API WebSocket messages before they enter the collector
    
    This is the "source of truth" - the original data that must be preserved
    exactly through the entire pipeline.
    """
    
    def __init__(self, framework: DeepEqualityFramework):
        self.framework = framework
        self.message_queue = Queue()
        self.running = False
        self.intercepted_count = 0
        
    def start_interception(self):
        """Start intercepting raw Polygon messages"""
        self.running = True
        # Note: In a real implementation, this would hook into the WebSocket
        # For now, we'll simulate or hook into the existing collector
        logger.info("🎯 PolygonAPIInterceptor started")
    
    def intercept_raw_message(self, raw_message: Dict[str, Any]) -> str:
        """
        Intercept and cache a raw Polygon API message
        
        Args:
            raw_message: Raw JSON message from Polygon API
            
        Returns:
            Message ID for tracking this message through pipeline
        """
        message_id = self.framework.start_tracking(raw_message)
        
        # Record the interception
        self.framework.record_stage(
            message_id, 
            "polygon_api", 
            raw_message,
            {
                "source": "polygon_websocket",
                "interception_time": time.time(),
                "raw_size_bytes": len(json.dumps(raw_message))
            }
        )
        
        self.intercepted_count += 1
        logger.debug(f"📡 Intercepted raw message #{self.intercepted_count}: {message_id}")
        
        return message_id
    
    def stop_interception(self):
        """Stop intercepting messages"""
        self.running = False
        logger.info(f"🛑 PolygonAPIInterceptor stopped. Total intercepted: {self.intercepted_count}")


class CollectorInterceptor:
    """
    Intercepts messages after collector processing but before binary protocol
    
    Captures the parsed and processed data from the Rust collector.
    """
    
    def __init__(self, framework: DeepEqualityFramework):
        self.framework = framework
        self.processed_count = 0
        
    def intercept_processed_message(self, message_id: str, processed_data: Dict[str, Any]) -> bool:
        """
        Intercept message after collector processing
        
        Args:
            message_id: Tracking ID for this message
            processed_data: Data after collector processing
            
        Returns:
            True if successfully recorded
        """
        success = self.framework.record_stage(
            message_id,
            "collector_output",
            processed_data,
            {
                "processing_time": time.time(),
                "data_size": len(str(processed_data)),
                "parsed_fields": list(processed_data.keys())
            }
        )
        
        if success:
            self.processed_count += 1
            logger.debug(f"🏭 Intercepted collector output #{self.processed_count}: {message_id}")
        
        return success


class BinaryProtocolInterceptor:
    """
    Intercepts messages in binary protocol format
    
    Monitors the Unix socket communication between collector and relay.
    """
    
    def __init__(self, framework: DeepEqualityFramework, socket_path: str = "/tmp/alphapulse/polygon.sock"):
        self.framework = framework
        self.socket_path = socket_path
        self.running = False
        self.monitor_thread = None
        self.binary_count = 0
        
    def start_monitoring(self):
        """Start monitoring the Unix socket for binary messages"""
        self.running = True
        self.monitor_thread = threading.Thread(target=self._monitor_unix_socket)
        self.monitor_thread.daemon = True
        self.monitor_thread.start()
        logger.info(f"🔌 BinaryProtocolInterceptor monitoring {self.socket_path}")
    
    def _monitor_unix_socket(self):
        """Monitor Unix socket in background thread"""
        try:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect(self.socket_path)
            sock.settimeout(1.0)
            
            while self.running:
                try:
                    # Read message header
                    header = sock.recv(8)
                    if len(header) != 8:
                        continue
                    
                    magic, msg_type, size, sequence = struct.unpack('<HHHH', header)
                    
                    if magic != 0x03FE:
                        continue
                    
                    # Read message body
                    body = sock.recv(size)
                    if len(body) != size:
                        continue
                    
                    # Process different message types
                    if msg_type == 1:  # TRADE message
                        self._intercept_trade_message(body, sequence)
                    elif msg_type == 8:  # SYMBOL_MAPPING message
                        self._intercept_symbol_mapping(body, sequence)
                    
                except socket.timeout:
                    continue
                except Exception as e:
                    if self.running:
                        logger.warning(f"Binary protocol monitoring error: {e}")
            
            sock.close()
            
        except Exception as e:
            logger.error(f"Failed to monitor binary protocol: {e}")
    
    def _intercept_trade_message(self, body: bytes, sequence: int):
        """Intercept and decode a binary trade message"""
        try:
            if len(body) < 64:
                return
            
            # Parse binary trade message
            fields = struct.unpack('<QQQQQQQf', body)
            
            binary_data = {
                "msg_type": "trade",
                "symbol_id": fields[0],
                "price_fixed": fields[1],
                "volume_fixed": fields[2],
                "liquidity_fixed": fields[3],
                "gas_cost_fixed": fields[4],
                "timestamp_ns": fields[5],
                "sequence": fields[6],
                "latency": fields[7],
                "price": fields[1] / 100000000.0,
                "volume": fields[2] / 100000000.0,
                "liquidity": fields[3] / 100000000.0,
                "gas_cost": fields[4] / 100000000.0
            }
            
            # For now, create a synthetic message ID since we don't have reverse lookup yet
            # In Phase 2, we'll enhance the protocol to include message IDs
            synthetic_id = f"binary_{sequence}_{int(time.time() * 1000000)}"
            
            self.framework.record_stage(
                synthetic_id,
                "binary_protocol",
                binary_data,
                {
                    "binary_size": len(body),
                    "sequence": sequence,
                    "decode_time": time.time()
                }
            )
            
            self.binary_count += 1
            
        except Exception as e:
            logger.warning(f"Failed to intercept binary trade message: {e}")
    
    def _intercept_symbol_mapping(self, body: bytes, sequence: int):
        """Intercept and decode a symbol mapping message"""
        try:
            # Parse symbol mapping
            symbol_id = struct.unpack('<Q', body[:8])[0]
            symbol_bytes = body[8:]
            null_pos = symbol_bytes.find(b'\x00')
            if null_pos >= 0:
                symbol = symbol_bytes[:null_pos].decode('utf-8')
            else:
                symbol = symbol_bytes.decode('utf-8')
            
            mapping_data = {
                "msg_type": "symbol_mapping",
                "symbol_id": symbol_id,
                "symbol": symbol
            }
            
            synthetic_id = f"mapping_{symbol_id}_{int(time.time() * 1000000)}"
            
            self.framework.record_stage(
                synthetic_id,
                "symbol_mapping",
                mapping_data,
                {
                    "symbol_id": symbol_id,
                    "symbol": symbol,
                    "sequence": sequence
                }
            )
            
        except Exception as e:
            logger.warning(f"Failed to intercept symbol mapping: {e}")
    
    def stop_monitoring(self):
        """Stop monitoring the Unix socket"""
        self.running = False
        if self.monitor_thread:
            self.monitor_thread.join(timeout=2.0)
        logger.info(f"🛑 BinaryProtocolInterceptor stopped. Messages processed: {self.binary_count}")


class WebSocketInterceptor:
    """
    Intercepts final WebSocket messages sent to the frontend
    
    This captures the final output of the pipeline that should exactly
    match the original input when reconstructed.
    """
    
    def __init__(self, framework: DeepEqualityFramework, websocket_uri: str = "ws://127.0.0.1:8765"):
        self.framework = framework
        self.websocket_uri = websocket_uri
        self.running = False
        self.monitor_task = None
        self.frontend_count = 0
        
    async def start_monitoring(self):
        """Start monitoring WebSocket messages to frontend"""
        self.running = True
        self.monitor_task = asyncio.create_task(self._monitor_websocket())
        logger.info(f"📱 WebSocketInterceptor monitoring {self.websocket_uri}")
    
    async def _monitor_websocket(self):
        """Monitor WebSocket messages in background task"""
        try:
            async with websockets.connect(self.websocket_uri) as websocket:
                while self.running:
                    try:
                        message = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                        data = json.loads(message)
                        
                        if data.get('msg_type') == 'trade':
                            await self._intercept_frontend_message(data)
                            
                    except asyncio.TimeoutError:
                        continue
                    except Exception as e:
                        if self.running:
                            logger.warning(f"WebSocket monitoring error: {e}")
                        break
                        
        except Exception as e:
            logger.error(f"Failed to monitor WebSocket: {e}")
    
    async def _intercept_frontend_message(self, data: Dict[str, Any]):
        """Intercept a frontend WebSocket message"""
        try:
            # For Phase 1, we'll use synthetic IDs
            # In Phase 2, we'll extract actual message IDs from the data
            synthetic_id = f"frontend_{data.get('symbol', 'unknown')}_{int(time.time() * 1000000)}"
            
            self.framework.record_stage(
                synthetic_id,
                "frontend_output",
                data,
                {
                    "websocket_time": time.time(),
                    "symbol": data.get('symbol', ''),
                    "message_type": data.get('msg_type', '')
                }
            )
            
            self.frontend_count += 1
            
        except Exception as e:
            logger.warning(f"Failed to intercept frontend message: {e}")
    
    async def stop_monitoring(self):
        """Stop monitoring WebSocket messages"""
        self.running = False
        if self.monitor_task:
            self.monitor_task.cancel()
            try:
                await self.monitor_task
            except asyncio.CancelledError:
                pass
        logger.info(f"🛑 WebSocketInterceptor stopped. Messages processed: {self.frontend_count}")


class PipelineInterceptor:
    """
    Main orchestrator that coordinates all pipeline interception
    
    Manages the complete message flow monitoring and provides a unified
    interface for tracking data through all pipeline stages.
    """
    
    def __init__(self, float_tolerance: float = 1e-10):
        self.framework = DeepEqualityFramework(float_tolerance)
        
        # Initialize stage interceptors
        self.polygon_interceptor = PolygonAPIInterceptor(self.framework)
        self.collector_interceptor = CollectorInterceptor(self.framework)
        self.binary_interceptor = BinaryProtocolInterceptor(self.framework)
        self.websocket_interceptor = WebSocketInterceptor(self.framework)
        
        # Tracking state
        self.is_monitoring = False
        self.message_callbacks: List[Callable] = []
        
    def start_full_monitoring(self):
        """Start monitoring all pipeline stages"""
        logger.info("🚀 Starting full pipeline monitoring...")
        
        self.polygon_interceptor.start_interception()
        self.binary_interceptor.start_monitoring()
        # Note: WebSocket monitoring will be started separately due to async nature
        
        self.is_monitoring = True
        logger.info("✅ Full pipeline monitoring active")
    
    async def start_async_monitoring(self):
        """Start async monitoring components (WebSocket)"""
        await self.websocket_interceptor.start_monitoring()
    
    def intercept_raw_polygon_message(self, raw_message: Dict[str, Any]) -> str:
        """Intercept a raw Polygon API message (entry point)"""
        return self.polygon_interceptor.intercept_raw_message(raw_message)
    
    def intercept_collector_output(self, message_id: str, processed_data: Dict[str, Any]) -> bool:
        """Intercept collector processed output"""
        return self.collector_interceptor.intercept_processed_message(message_id, processed_data)
    
    def validate_final_output(self, message_id: str, frontend_output: Dict[str, Any]) -> Dict[str, Any]:
        """Validate final frontend output against original"""
        return self.framework.validate_final_output(message_id, frontend_output)
    
    def add_message_callback(self, callback: Callable[[InterceptedMessage], None]):
        """Add callback for when messages are intercepted"""
        self.message_callbacks.append(callback)
    
    def get_pipeline_statistics(self) -> Dict[str, Any]:
        """Get comprehensive pipeline monitoring statistics"""
        framework_stats = self.framework.get_statistics()
        
        return {
            "framework": framework_stats,
            "stages": {
                "polygon_api": {
                    "intercepted_count": self.polygon_interceptor.intercepted_count
                },
                "collector": {
                    "processed_count": self.collector_interceptor.processed_count
                },
                "binary_protocol": {
                    "binary_count": self.binary_interceptor.binary_count
                },
                "frontend": {
                    "frontend_count": self.websocket_interceptor.frontend_count
                }
            },
            "monitoring_active": self.is_monitoring
        }
    
    def stop_monitoring(self):
        """Stop all pipeline monitoring"""
        logger.info("🛑 Stopping pipeline monitoring...")
        
        self.polygon_interceptor.stop_interception()
        self.binary_interceptor.stop_monitoring()
        
        self.is_monitoring = False
        logger.info("✅ Pipeline monitoring stopped")
    
    async def stop_async_monitoring(self):
        """Stop async monitoring components"""
        await self.websocket_interceptor.stop_monitoring()
    
    def get_message_trace(self, message_id: str) -> Optional[Dict[str, Any]]:
        """Get detailed trace for a specific message"""
        return self.framework.get_detailed_trace(message_id)
    
    def find_message_by_characteristics(self, **kwargs) -> List[str]:
        """
        Find message IDs by characteristics
        
        Args:
            **kwargs: Search criteria (symbol, price_range, etc.)
            
        Returns:
            List of matching message IDs
        """
        # This would implement search functionality across tracked messages
        # For now, return empty list - will be enhanced in Phase 2
        return []


class ValidationSession:
    """
    Manages a complete validation session with start/stop lifecycle
    
    Provides high-level interface for running deep equality validation
    on live pipeline data.
    """
    
    def __init__(self, session_id: Optional[str] = None, float_tolerance: float = 1e-10):
        self.session_id = session_id or f"session_{int(time.time())}"
        self.interceptor = PipelineInterceptor(float_tolerance)
        self.start_time = None
        self.end_time = None
        self.session_results: List[Dict] = []
        
    async def start_session(self, duration_seconds: Optional[int] = None):
        """
        Start a validation session
        
        Args:
            duration_seconds: How long to run (None = indefinite)
        """
        logger.info(f"🎯 Starting validation session: {self.session_id}")
        self.start_time = time.time()
        
        # Start monitoring
        self.interceptor.start_full_monitoring()
        await self.interceptor.start_async_monitoring()
        
        # Run for specified duration or until stopped
        if duration_seconds:
            await asyncio.sleep(duration_seconds)
            await self.stop_session()
    
    async def stop_session(self):
        """Stop the validation session and generate report"""
        logger.info(f"🏁 Stopping validation session: {self.session_id}")
        self.end_time = time.time()
        
        # Stop monitoring
        self.interceptor.stop_monitoring()
        await self.interceptor.stop_async_monitoring()
        
        # Generate session report
        session_duration = self.end_time - self.start_time
        stats = self.interceptor.get_pipeline_statistics()
        
        session_report = {
            "session_id": self.session_id,
            "start_time": self.start_time,
            "end_time": self.end_time,
            "duration_seconds": session_duration,
            "statistics": stats,
            "total_validations": len(self.session_results),
            "successful_validations": sum(1 for r in self.session_results if r.get("is_equal", False))
        }
        
        logger.info(f"📊 Session {self.session_id} complete: {session_report}")
        return session_report
    
    def validate_message(self, message_id: str, frontend_output: Dict[str, Any]) -> Dict[str, Any]:
        """Validate a message and add to session results"""
        result = self.interceptor.validate_final_output(message_id, frontend_output)
        self.session_results.append(result)
        return result