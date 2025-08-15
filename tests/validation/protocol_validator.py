#!/usr/bin/env python3
"""
Protocol Validator
Captures data from both Unix socket (binary) and WebSocket bridge (JSON) for comparison
"""

import asyncio
import json
import socket
import struct
import time
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass, field
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Protocol constants
MAGIC_BYTE = 0xFE
UNIX_SOCKET_PATH = "/tmp/alphapulse/relay.sock"

# Message types
MSG_TYPE_TRADE = 1
MSG_TYPE_ORDERBOOK = 2
MSG_TYPE_HEARTBEAT = 3
MSG_TYPE_METRICS = 4
MSG_TYPE_L2_SNAPSHOT = 5
MSG_TYPE_L2_DELTA = 6
MSG_TYPE_L2_RESET = 7
MSG_TYPE_SYMBOL_MAPPING = 8


@dataclass
class BinaryMessage:
    """Represents a decoded binary protocol message"""
    timestamp: float  # Capture time
    msg_type: int
    sequence: int
    symbol_hash: Optional[int] = None
    price_raw: Optional[int] = None  # Fixed-point (8 decimals)
    price_float: Optional[float] = None
    volume_raw: Optional[int] = None
    volume_float: Optional[float] = None
    side: Optional[int] = None
    latency_ns: Optional[int] = None
    raw_bytes: bytes = field(default_factory=bytes)
    # Additional fields for other message types
    symbol_str: Optional[str] = None
    l2_sequence: Optional[int] = None
    l2_data: Optional[Dict] = None
    l2_updates: Optional[List] = None
    heartbeat_timestamp: Optional[int] = None
    heartbeat_sequence: Optional[int] = None


class BinaryProtocolReader:
    """Reads and decodes binary protocol messages from Unix socket"""
    
    def __init__(self, socket_path: str = UNIX_SOCKET_PATH):
        self.socket_path = socket_path
        self.messages: List[BinaryMessage] = []
        self.is_running = False
        self.stats = {
            'total_messages': 0,
            'trade_messages': 0,
            'orderbook_messages': 0,
            'heartbeat_messages': 0,
            'symbol_mappings': 0,
            'decode_errors': 0
        }
    
    def connect_and_capture(self, duration: int = 60) -> List[BinaryMessage]:
        """Connect to Unix socket and capture binary messages"""
        start_time = time.time()
        self.is_running = True
        
        try:
            # Create Unix socket connection
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect(self.socket_path)
            sock.settimeout(1.0)  # 1 second timeout for non-blocking reads
            
            logger.info(f"Connected to Unix socket: {self.socket_path}")
            
            buffer = bytearray()
            end_time = start_time + duration
            
            while self.is_running and time.time() < end_time:
                try:
                    # Read data from socket
                    data = sock.recv(4096)
                    if not data:
                        logger.warning("Socket closed by remote")
                        break
                    
                    buffer.extend(data)
                    
                    # Process complete messages from buffer
                    while len(buffer) >= 8:  # Minimum header size
                        # Try to decode a message
                        msg, consumed = self._decode_message(buffer)
                        if msg:
                            self.messages.append(msg)
                            self._update_stats(msg)
                            buffer = buffer[consumed:]
                            
                            # Log progress
                            if len(self.messages) % 100 == 0:
                                logger.info(f"Captured {len(self.messages)} binary messages")
                        else:
                            break  # Not enough data for complete message
                            
                except socket.timeout:
                    continue
                except Exception as e:
                    logger.error(f"Error reading from socket: {e}")
                    self.stats['decode_errors'] += 1
                    
        except Exception as e:
            logger.error(f"Socket connection error: {e}")
            raise
        finally:
            sock.close()
            self.is_running = False
            elapsed = time.time() - start_time
            logger.info(f"Binary capture complete: {len(self.messages)} messages in {elapsed:.2f}s")
            
        return self.messages
    
    def _decode_message(self, buffer: bytearray) -> Tuple[Optional[BinaryMessage], int]:
        """Decode a single message from buffer"""
        if len(buffer) < 8:
            return None, 0
        
        # Parse header (8 bytes)
        magic = buffer[0]
        if magic != MAGIC_BYTE:
            logger.error(f"Invalid magic byte: {magic:02x}")
            # Try to find next valid magic byte
            for i in range(1, min(len(buffer), 100)):
                if buffer[i] == MAGIC_BYTE:
                    return None, i  # Skip to next potential message
            return None, 1  # Skip one byte and try again
        
        msg_type = buffer[1]
        flags = buffer[2]
        length = struct.unpack('<H', buffer[3:5])[0]
        sequence = struct.unpack('<I', bytes(buffer[5:8]) + b'\x00')[0]  # 3 bytes -> 4 bytes
        
        # Check if we have complete message
        total_size = 8 + length
        if len(buffer) < total_size:
            return None, 0  # Wait for more data
        
        # Extract payload
        payload = buffer[8:total_size]
        capture_time = time.time()
        
        # Decode based on message type
        msg = BinaryMessage(
            timestamp=capture_time,
            msg_type=msg_type,
            sequence=sequence,
            raw_bytes=bytes(buffer[:total_size])
        )
        
        if msg_type == MSG_TYPE_TRADE and len(payload) >= 64:
            # Trade message: 64 bytes
            # timestamp_ns(8) + ingestion_ns(8) + relay_ns(8) + bridge_ns(8) + 
            # price(8) + volume(8) + symbol_hash(8) + side(1) + padding(7)
            msg.symbol_hash = struct.unpack('<Q', payload[48:56])[0]
            msg.price_raw = struct.unpack('<Q', payload[32:40])[0]
            msg.price_float = msg.price_raw / 1e8  # Convert from fixed-point
            msg.volume_raw = struct.unpack('<Q', payload[40:48])[0]
            msg.volume_float = msg.volume_raw / 1e8
            msg.side = payload[56] if len(payload) > 56 else None
            
            # Extract latency info
            timestamp_ns = struct.unpack('<Q', payload[0:8])[0]
            ingestion_ns = struct.unpack('<Q', payload[8:16])[0]
            if ingestion_ns > 0:
                msg.latency_ns = ingestion_ns - timestamp_ns
                
        elif msg_type == MSG_TYPE_ORDERBOOK:
            # OrderBook message decoding
            if len(payload) >= 24:
                msg.symbol_hash = struct.unpack('<Q', payload[16:24])[0]
                # Additional orderbook fields would be parsed here
                
        elif msg_type == MSG_TYPE_L2_SNAPSHOT:
            # L2 Snapshot message decoding
            if len(payload) >= 32:
                msg.symbol_hash = struct.unpack('<Q', payload[16:24])[0]
                msg.l2_sequence = struct.unpack('<Q', payload[24:32])[0]
                # Parse bid/ask levels
                msg.l2_data = self._parse_l2_levels(payload[32:])
                
        elif msg_type == MSG_TYPE_L2_DELTA:
            # L2 Delta message decoding
            if len(payload) >= 32:
                msg.symbol_hash = struct.unpack('<Q', payload[16:24])[0]
                msg.l2_sequence = struct.unpack('<Q', payload[24:32])[0]
                # Parse updates
                msg.l2_updates = self._parse_l2_updates(payload[32:])
                
        elif msg_type == MSG_TYPE_SYMBOL_MAPPING and len(payload) >= 10:
            # Symbol mapping: hash(8) + length(2) + string
            msg.symbol_hash = struct.unpack('<Q', payload[0:8])[0]
            str_len = struct.unpack('<H', payload[8:10])[0]
            if len(payload) >= 10 + str_len:
                msg.symbol_str = payload[10:10+str_len].decode('utf-8', errors='ignore')
                
        elif msg_type == MSG_TYPE_HEARTBEAT:
            # Heartbeat message
            if len(payload) >= 16:
                msg.heartbeat_timestamp = struct.unpack('<Q', payload[0:8])[0]
                msg.heartbeat_sequence = struct.unpack('<Q', payload[8:16])[0]
            
        return msg, total_size
    
    def _parse_l2_levels(self, data: bytes) -> Dict[str, List]:
        """Parse L2 order book levels from binary data"""
        levels = {'bids': [], 'asks': []}
        if len(data) < 4:
            return levels
            
        num_bids = struct.unpack('<H', data[0:2])[0]
        num_asks = struct.unpack('<H', data[2:4])[0]
        offset = 4
        
        # Parse bids
        for _ in range(min(num_bids, 50)):  # Limit to 50 levels
            if offset + 16 > len(data):
                break
            price = struct.unpack('<Q', data[offset:offset+8])[0] / 1e8
            size = struct.unpack('<Q', data[offset+8:offset+16])[0] / 1e8
            levels['bids'].append({'price': price, 'size': size})
            offset += 16
            
        # Parse asks
        for _ in range(min(num_asks, 50)):
            if offset + 16 > len(data):
                break
            price = struct.unpack('<Q', data[offset:offset+8])[0] / 1e8
            size = struct.unpack('<Q', data[offset+8:offset+16])[0] / 1e8
            levels['asks'].append({'price': price, 'size': size})
            offset += 16
            
        return levels
    
    def _parse_l2_updates(self, data: bytes) -> List[Dict]:
        """Parse L2 delta updates from binary data"""
        updates = []
        if len(data) < 2:
            return updates
            
        num_updates = struct.unpack('<H', data[0:2])[0]
        offset = 2
        
        for _ in range(min(num_updates, 100)):  # Limit to 100 updates
            if offset + 17 > len(data):
                break
            
            side = data[offset]  # 0=bid, 1=ask
            action = data[offset+1]  # 0=delete, 1=update, 2=insert
            price = struct.unpack('<Q', data[offset+2:offset+10])[0] / 1e8
            size = struct.unpack('<Q', data[offset+10:offset+18])[0] / 1e8
            
            updates.append({
                'side': 'bid' if side == 0 else 'ask',
                'action': ['delete', 'update', 'insert'][action] if action < 3 else 'unknown',
                'price': price,
                'size': size
            })
            offset += 18
            
        return updates
    
    def _update_stats(self, msg: BinaryMessage):
        """Update statistics"""
        self.stats['total_messages'] += 1
        
        if msg.msg_type == MSG_TYPE_TRADE:
            self.stats['trade_messages'] += 1
        elif msg.msg_type == MSG_TYPE_ORDERBOOK:
            self.stats['orderbook_messages'] += 1
        elif msg.msg_type == MSG_TYPE_HEARTBEAT:
            self.stats['heartbeat_messages'] += 1
        elif msg.msg_type == MSG_TYPE_SYMBOL_MAPPING:
            self.stats['symbol_mappings'] += 1
        elif msg.msg_type == MSG_TYPE_L2_SNAPSHOT:
            self.stats.setdefault('l2_snapshots', 0)
            self.stats['l2_snapshots'] += 1
        elif msg.msg_type == MSG_TYPE_L2_DELTA:
            self.stats.setdefault('l2_deltas', 0)
            self.stats['l2_deltas'] += 1
    
    def get_trades_by_hash(self, symbol_hash: int) -> List[BinaryMessage]:
        """Get all trade messages for a specific symbol hash"""
        return [msg for msg in self.messages 
                if msg.msg_type == MSG_TYPE_TRADE and msg.symbol_hash == symbol_hash]
    
    def get_price_series(self, symbol_hash: int) -> List[Tuple[float, float]]:
        """Get time series of prices for a symbol hash"""
        series = []
        for msg in self.messages:
            if msg.msg_type == MSG_TYPE_TRADE and msg.symbol_hash == symbol_hash and msg.price_float:
                series.append((msg.timestamp, msg.price_float))
        return sorted(series, key=lambda x: x[0])
    
    def save_to_file(self, filepath: str):
        """Save captured binary messages to JSON"""
        data = {
            'capture_info': {
                'socket_path': self.socket_path,
                'total_messages': len(self.messages)
            },
            'statistics': self.stats,
            'messages': [
                {
                    'timestamp': msg.timestamp,
                    'msg_type': msg.msg_type,
                    'sequence': msg.sequence,
                    'symbol_hash': msg.symbol_hash,
                    'price_raw': msg.price_raw,
                    'price_float': msg.price_float,
                    'volume_raw': msg.volume_raw,
                    'volume_float': msg.volume_float,
                    'side': msg.side,
                    'latency_ns': msg.latency_ns,
                    'raw_hex': msg.raw_bytes.hex() if msg.raw_bytes else None
                }
                for msg in self.messages
            ]
        }
        
        with open(filepath, 'w') as f:
            json.dump(data, f, indent=2)
        logger.info(f"Saved {len(self.messages)} binary messages to {filepath}")


class ProtocolValidator:
    """Validates consistency between binary protocol and JSON output"""
    
    def __init__(self):
        self.binary_messages: List[BinaryMessage] = []
        self.json_messages: List[Dict] = []
        self.symbol_mappings: Dict[int, str] = {}
        self.validation_results: List[Dict] = []
    
    def validate_price_accuracy(self, tolerance: float = 0.0001) -> List[Dict]:
        """Validate that prices match between binary and JSON with given tolerance"""
        results = []
        
        # Build mapping of symbol hashes to names from JSON
        for json_msg in self.json_messages:
            if json_msg.get('msg_type') == 'symbol_mapping':
                hash_str = json_msg.get('symbol_hash')
                symbol = json_msg.get('symbol')
                if hash_str and symbol:
                    self.symbol_mappings[int(hash_str)] = symbol
        
        # Group messages by symbol and timestamp
        binary_trades = {}
        json_trades = {}
        
        for msg in self.binary_messages:
            if msg.msg_type == MSG_TYPE_TRADE and msg.symbol_hash and msg.price_float:
                key = (msg.symbol_hash, int(msg.timestamp))  # Group by hash and second
                if key not in binary_trades:
                    binary_trades[key] = []
                binary_trades[key].append(msg)
        
        for msg in self.json_messages:
            if msg.get('msg_type') == 'trade':
                symbol = msg.get('symbol')
                symbol_hash = int(msg.get('symbol_hash', 0))
                price = msg.get('price')
                timestamp = msg.get('timestamp', 0) / 1000  # Convert ms to seconds
                
                if symbol_hash and price:
                    key = (symbol_hash, int(timestamp))
                    if key not in json_trades:
                        json_trades[key] = []
                    json_trades[key].append({
                        'symbol': symbol,
                        'price': price,
                        'volume': msg.get('volume'),
                        'timestamp': timestamp
                    })
        
        # Compare prices
        for key in set(binary_trades.keys()) & set(json_trades.keys()):
            symbol_hash, timestamp = key
            symbol = self.symbol_mappings.get(symbol_hash, f"hash_{symbol_hash}")
            
            for binary_msg in binary_trades[key]:
                # Find closest JSON message
                best_match = None
                best_diff = float('inf')
                
                for json_trade in json_trades[key]:
                    price_diff = abs(binary_msg.price_float - json_trade['price'])
                    if price_diff < best_diff:
                        best_diff = price_diff
                        best_match = json_trade
                
                if best_match:
                    relative_diff = best_diff / binary_msg.price_float if binary_msg.price_float > 0 else 0
                    passed = relative_diff <= tolerance
                    
                    results.append({
                        'symbol': symbol,
                        'symbol_hash': symbol_hash,
                        'timestamp': timestamp,
                        'binary_price': binary_msg.price_float,
                        'json_price': best_match['price'],
                        'absolute_diff': best_diff,
                        'relative_diff': relative_diff,
                        'passed': passed,
                        'binary_volume': binary_msg.volume_float,
                        'json_volume': best_match.get('volume')
                    })
        
        return results
    
    def validate_latency(self) -> Dict[str, Any]:
        """Validate latency measurements"""
        latencies = []
        
        for msg in self.binary_messages:
            if msg.msg_type == MSG_TYPE_TRADE and msg.latency_ns:
                latencies.append(msg.latency_ns / 1e6)  # Convert to milliseconds
        
        if not latencies:
            return {'error': 'No latency data found'}
        
        return {
            'min_ms': min(latencies),
            'max_ms': max(latencies),
            'avg_ms': sum(latencies) / len(latencies),
            'count': len(latencies),
            'p50_ms': sorted(latencies)[len(latencies) // 2],
            'p95_ms': sorted(latencies)[int(len(latencies) * 0.95)] if len(latencies) > 20 else max(latencies)
        }
    
    def generate_report(self) -> Dict[str, Any]:
        """Generate validation report"""
        price_validations = self.validate_price_accuracy()
        latency_stats = self.validate_latency()
        
        passed = sum(1 for v in price_validations if v['passed'])
        failed = len(price_validations) - passed
        
        return {
            'summary': {
                'total_binary_messages': len(self.binary_messages),
                'total_json_messages': len(self.json_messages),
                'validations_performed': len(price_validations),
                'passed': passed,
                'failed': failed,
                'pass_rate': passed / len(price_validations) if price_validations else 0
            },
            'latency_stats': latency_stats,
            'failed_validations': [v for v in price_validations if not v['passed']][:10],  # First 10 failures
            'sample_validations': price_validations[:5]  # Sample of validations
        }


def main():
    """Example usage"""
    # Capture binary data
    binary_reader = BinaryProtocolReader()
    binary_messages = binary_reader.connect_and_capture(duration=30)
    
    # Load JSON data (from ws_data_interceptor output)
    # In real test, this would be captured simultaneously
    
    # Validate
    validator = ProtocolValidator()
    validator.binary_messages = binary_messages
    # validator.json_messages = json_messages  # Load from ws_data_interceptor
    
    report = validator.generate_report()
    print(json.dumps(report, indent=2))
    
    # Save results
    binary_reader.save_to_file("binary_capture.json")


if __name__ == "__main__":
    main()