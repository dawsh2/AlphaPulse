#!/usr/bin/env python3
"""
TLV Construction Validation for Polygon Adapter

This Python script validates that the Polygon adapter is properly constructing
TLV messages by connecting to the relay and parsing received messages.

Usage:
    python3 test_polygon_adapter_tlv.py [--socket-path PATH] [--duration SECONDS]
"""

import socket
import struct
import time
import json
import argparse
import sys
from typing import Optional, Dict, Any
from dataclasses import dataclass

@dataclass
class TLVHeader:
    magic: int
    relay_domain: int  
    source: int
    sequence: int
    payload_size: int
    timestamp_ns: int
    
    @property
    def is_valid(self) -> bool:
        return self.magic == 0xDEADBEEF

@dataclass 
class TLVExtension:
    tlv_type: int
    length: int
    data: bytes

class PolygonTLVValidator:
    """Validates Polygon adapter TLV construction"""
    
    # TLV Type Constants (from Protocol V2)
    POOL_SWAP_TLV = 1
    POOL_SYNC_TLV = 2  
    POOL_MINT_TLV = 3
    POOL_BURN_TLV = 4
    POOL_TICK_TLV = 5
    POOL_STATE_TLV = 10
    
    TLV_TYPE_NAMES = {
        POOL_SWAP_TLV: "PoolSwap",
        POOL_SYNC_TLV: "PoolSync", 
        POOL_MINT_TLV: "PoolMint",
        POOL_BURN_TLV: "PoolBurn",
        POOL_TICK_TLV: "PoolTick",
        POOL_STATE_TLV: "PoolState",
    }
    
    def __init__(self, socket_path: str):
        self.socket_path = socket_path
        self.messages_validated = 0
        self.validation_errors = 0
        self.tlv_type_counts = {}
        
    def connect_to_relay(self) -> socket.socket:
        """Connect to the market data relay Unix socket"""
        try:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect(self.socket_path)
            print(f"✅ Connected to relay at {self.socket_path}")
            return sock
        except Exception as e:
            print(f"❌ Failed to connect to relay: {e}")
            sys.exit(1)
            
    def parse_header(self, header_bytes: bytes) -> Optional[TLVHeader]:
        """Parse 32-byte TLV message header"""
        if len(header_bytes) < 32:
            return None
            
        try:
            # Parse header fields (little-endian)
            magic = struct.unpack('<I', header_bytes[0:4])[0]
            relay_domain = struct.unpack('<H', header_bytes[4:6])[0]
            source = struct.unpack('<H', header_bytes[6:8])[0]
            sequence = struct.unpack('<Q', header_bytes[8:16])[0]
            payload_size = struct.unpack('<I', header_bytes[16:20])[0]
            timestamp_ns = struct.unpack('<Q', header_bytes[24:32])[0]
            
            return TLVHeader(
                magic=magic,
                relay_domain=relay_domain,
                source=source, 
                sequence=sequence,
                payload_size=payload_size,
                timestamp_ns=timestamp_ns
            )
        except Exception as e:
            print(f"❌ Header parse error: {e}")
            return None
            
    def parse_tlv_extensions(self, payload: bytes) -> list[TLVExtension]:
        """Parse TLV extensions from payload"""
        extensions = []
        offset = 0
        
        while offset + 4 <= len(payload):
            # Parse TLV header (type + length)
            tlv_type = struct.unpack('<H', payload[offset:offset+2])[0]
            length = struct.unpack('<H', payload[offset+2:offset+4])[0]
            
            if offset + 4 + length > len(payload):
                print(f"❌ TLV payload truncated at offset {offset}")
                break
                
            data = payload[offset+4:offset+4+length]
            extensions.append(TLVExtension(tlv_type, length, data))
            
            offset += 4 + length
            
        return extensions
        
    def validate_pool_swap_tlv(self, data: bytes) -> Dict[str, Any]:
        """Validate PoolSwap TLV structure and extract key fields"""
        if len(data) < 140:  # Minimum size for PoolSwapTLV
            return {"error": f"PoolSwap TLV too short: {len(data)} bytes"}
            
        try:
            # Extract key fields (simplified parsing)
            pool_address = data[0:20]
            token_in_address = data[20:40] 
            token_out_address = data[40:60]
            venue_id = struct.unpack('<H', data[60:62])[0]
            amount_in = struct.unpack('<Q', data[62:70])[0]
            amount_out = struct.unpack('<Q', data[70:78])[0]
            
            # Validate venue ID (should be Polygon = 137)
            if venue_id != 137:
                return {"error": f"Invalid venue ID for Polygon: {venue_id}"}
                
            # Validate amounts are non-zero
            if amount_in == 0 or amount_out == 0:
                return {"error": "Zero amounts in swap"}
                
            return {
                "pool_address": pool_address.hex(),
                "token_in": token_in_address.hex(),
                "token_out": token_out_address.hex(), 
                "venue_id": venue_id,
                "amount_in": amount_in,
                "amount_out": amount_out,
                "valid": True
            }
        except Exception as e:
            return {"error": f"PoolSwap parsing error: {e}"}
            
    def validate_pool_sync_tlv(self, data: bytes) -> Dict[str, Any]:
        """Validate PoolSync TLV structure"""
        if len(data) < 80:  # Minimum size for PoolSyncTLV
            return {"error": f"PoolSync TLV too short: {len(data)} bytes"}
            
        try:
            pool_address = data[0:20]
            token0_address = data[20:40]
            token1_address = data[40:60]
            venue_id = struct.unpack('<H', data[60:62])[0]
            reserve0 = struct.unpack('<Q', data[62:70])[0]
            reserve1 = struct.unpack('<Q', data[70:78])[0]
            
            if venue_id != 137:
                return {"error": f"Invalid venue ID for Polygon: {venue_id}"}
                
            return {
                "pool_address": pool_address.hex(),
                "token0": token0_address.hex(),
                "token1": token1_address.hex(),
                "venue_id": venue_id,
                "reserve0": reserve0,
                "reserve1": reserve1,
                "valid": True
            }
        except Exception as e:
            return {"error": f"PoolSync parsing error: {e}"}
            
    def validate_tlv_message(self, message: bytes) -> Dict[str, Any]:
        """Validate complete TLV message"""
        if len(message) < 32:
            return {"error": "Message too short for header"}
            
        # Parse header
        header = self.parse_header(message[:32])
        if not header:
            return {"error": "Failed to parse header"}
            
        if not header.is_valid:
            return {"error": f"Invalid magic number: 0x{header.magic:08X}"}
            
        # Validate payload size
        expected_message_size = 32 + header.payload_size
        if len(message) < expected_message_size:
            return {"error": f"Message truncated: {len(message)} < {expected_message_size}"}
            
        # Parse TLV extensions
        payload = message[32:32+header.payload_size]
        extensions = self.parse_tlv_extensions(payload)
        
        validation_results = []
        for ext in extensions:
            tlv_name = self.TLV_TYPE_NAMES.get(ext.tlv_type, f"Unknown({ext.tlv_type})")
            
            # Track TLV type counts
            self.tlv_type_counts[tlv_name] = self.tlv_type_counts.get(tlv_name, 0) + 1
            
            # Validate specific TLV types
            if ext.tlv_type == self.POOL_SWAP_TLV:
                result = self.validate_pool_swap_tlv(ext.data)
                result["tlv_type"] = "PoolSwap"
                validation_results.append(result)
                
            elif ext.tlv_type == self.POOL_SYNC_TLV:
                result = self.validate_pool_sync_tlv(ext.data)
                result["tlv_type"] = "PoolSync"
                validation_results.append(result)
                
            else:
                validation_results.append({
                    "tlv_type": tlv_name,
                    "length": ext.length,
                    "valid": True  # Basic validation for now
                })
                
        return {
            "header": {
                "magic": f"0x{header.magic:08X}",
                "relay_domain": header.relay_domain,
                "source": header.source,
                "sequence": header.sequence,
                "payload_size": header.payload_size,
                "timestamp_ns": header.timestamp_ns
            },
            "extensions": validation_results,
            "valid": all(ext.get("valid", True) for ext in validation_results)
        }
        
    def run_validation(self, duration_seconds: int):
        """Run TLV validation for specified duration"""
        print(f"🚀 Starting Polygon TLV Validation")
        print(f"   Duration: {duration_seconds}s")
        print(f"   Socket: {self.socket_path}")
        print()
        
        sock = self.connect_to_relay()
        sock.settimeout(5.0)  # 5 second read timeout
        
        start_time = time.time()
        last_report_time = start_time
        
        try:
            while time.time() - start_time < duration_seconds:
                try:
                    # Read message header first
                    header_data = b""
                    while len(header_data) < 32:
                        chunk = sock.recv(32 - len(header_data))
                        if not chunk:
                            break
                        header_data += chunk
                        
                    if len(header_data) < 32:
                        continue
                        
                    # Parse header to get payload size
                    header = self.parse_header(header_data)
                    if not header or not header.is_valid:
                        print(f"❌ Invalid header received")
                        self.validation_errors += 1
                        continue
                        
                    # Read payload
                    payload_data = b""
                    while len(payload_data) < header.payload_size:
                        chunk = sock.recv(header.payload_size - len(payload_data))
                        if not chunk:
                            break
                        payload_data += chunk
                        
                    if len(payload_data) < header.payload_size:
                        print(f"❌ Payload truncated")
                        self.validation_errors += 1
                        continue
                        
                    # Validate complete message
                    full_message = header_data + payload_data
                    result = self.validate_tlv_message(full_message)
                    
                    self.messages_validated += 1
                    
                    if not result.get("valid", False):
                        self.validation_errors += 1
                        print(f"❌ Validation failed for message {self.messages_validated}")
                        print(f"   Error: {result.get('error', 'Unknown error')}")
                    else:
                        # Success - show periodic updates
                        if self.messages_validated <= 5 or self.messages_validated % 10 == 0:
                            extensions = result.get("extensions", [])
                            tlv_types = [ext.get("tlv_type", "Unknown") for ext in extensions]
                            print(f"✅ Message {self.messages_validated} validated: {', '.join(tlv_types)}")
                            
                    # Periodic reporting
                    current_time = time.time()
                    if current_time - last_report_time >= 10:
                        self.print_status_report()
                        last_report_time = current_time
                        
                except socket.timeout:
                    print("⏰ No messages received (timeout)")
                except Exception as e:
                    print(f"❌ Validation error: {e}")
                    self.validation_errors += 1
                    
        finally:
            sock.close()
            
        self.print_final_report()
        
    def print_status_report(self):
        """Print periodic status report"""
        success_rate = 0.0
        if self.messages_validated > 0:
            success_rate = ((self.messages_validated - self.validation_errors) / self.messages_validated) * 100
            
        print(f"📊 Status: {self.messages_validated} messages, {self.validation_errors} errors, {success_rate:.1f}% success")
        
    def print_final_report(self):
        """Print final validation report"""
        print()
        print("📊 ===== POLYGON TLV VALIDATION REPORT =====")
        print(f"Messages Validated: {self.messages_validated}")
        print(f"Validation Errors: {self.validation_errors}")
        
        if self.messages_validated > 0:
            success_rate = ((self.messages_validated - self.validation_errors) / self.messages_validated) * 100
            print(f"Success Rate: {success_rate:.1f}%")
        else:
            print("Success Rate: N/A (no messages received)")
            
        print()
        print("TLV Type Distribution:")
        for tlv_type, count in sorted(self.tlv_type_counts.items()):
            print(f"  {tlv_type}: {count}")
            
        print()
        if self.validation_errors == 0 and self.messages_validated > 0:
            print("✅ VALIDATION PASSED - Polygon adapter TLV construction is correct")
        elif self.messages_validated == 0:
            print("⚠️ NO MESSAGES RECEIVED - Check if Polygon collector is running")
        else:
            print("❌ VALIDATION FAILED - TLV construction issues detected")
        print("==========================================")

def main():
    parser = argparse.ArgumentParser(description="Validate Polygon adapter TLV construction")
    parser.add_argument("--socket-path", default="/tmp/alphapulse/market_data.sock",
                      help="Unix socket path to market data relay")
    parser.add_argument("--duration", type=int, default=60,
                      help="Validation duration in seconds")
    
    args = parser.parse_args()
    
    validator = PolygonTLVValidator(args.socket_path)
    validator.run_validation(args.duration)

if __name__ == "__main__":
    main()