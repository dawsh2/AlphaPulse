#!/usr/bin/env python3
"""Test script to verify orderbook deltas are being written to shared memory"""
import time
import struct
import mmap
import os
from typing import List, Optional

def test_delta_shared_memory():
    """Test reading orderbook deltas from shared memory"""
    print("🧪 Testing OrderBook Delta Shared Memory")
    print("=" * 50)
    
    shm_path = "/tmp/alphapulse_shm/orderbook_deltas"
    
    # Check if shared memory file exists
    if not os.path.exists(shm_path):
        print(f"❌ Shared memory file not found: {shm_path}")
        print("   Make sure collectors are running with shared memory enabled")
        return
    
    try:
        # Open the shared memory file
        with open(shm_path, "rb") as f:
            # Memory map the file
            with mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as mm:
                # Read the ring buffer header (first part of file)
                header_size = 256  # Simplified estimate
                
                # Ring buffer header structure (simplified)
                # version(4) + capacity(4) + write_sequence(8) + cached_write_sequence(8)
                # + writer_pid(4) + last_write_ns(8) + reader_cursors(16*8) + padding
                
                if len(mm) < header_size:
                    print(f"❌ File too small: {len(mm)} bytes")
                    return
                
                # Read header fields
                version = struct.unpack('<I', mm[0:4])[0]
                capacity = struct.unpack('<I', mm[4:8])[0]
                write_sequence = struct.unpack('<Q', mm[8:16])[0]
                cached_write_sequence = struct.unpack('<Q', mm[16:24])[0]
                writer_pid = struct.unpack('<I', mm[24:28])[0]
                last_write_ns = struct.unpack('<Q', mm[28:36])[0]
                
                print(f"📊 Shared Memory Header:")
                print(f"   Version: {version}")
                print(f"   Capacity: {capacity}")
                print(f"   Write Sequence: {write_sequence}")
                print(f"   Cached Write Sequence: {cached_write_sequence}")
                print(f"   Writer PID: {writer_pid}")
                print(f"   Last Write (ns): {last_write_ns}")
                
                if write_sequence == 0:
                    print("⏳ No deltas written yet")
                    print("   Waiting for collectors to generate orderbook deltas...")
                    return
                
                # Calculate data start position
                data_start = header_size
                delta_size = 256  # SharedOrderBookDelta::SIZE
                
                print(f"\n📈 Reading Deltas:")
                print(f"   Data starts at offset: {data_start}")
                print(f"   Delta size: {delta_size} bytes")
                print(f"   Available deltas: {min(write_sequence, capacity)}")
                
                # Read the last few deltas
                num_to_read = min(5, write_sequence)
                for i in range(num_to_read):
                    # Calculate index (most recent deltas)
                    index = (write_sequence - 1 - i) % capacity
                    offset = data_start + (index * delta_size)
                    
                    if offset + delta_size <= len(mm):
                        # Read SharedOrderBookDelta structure
                        delta_data = mm[offset:offset + delta_size]
                        
                        # Parse delta structure (simplified)
                        timestamp_ns = struct.unpack('<Q', delta_data[0:8])[0]
                        symbol = delta_data[8:24].rstrip(b'\x00').decode('utf-8', errors='ignore')
                        exchange = delta_data[24:40].rstrip(b'\x00').decode('utf-8', errors='ignore')
                        version = struct.unpack('<Q', delta_data[40:48])[0]
                        prev_version = struct.unpack('<Q', delta_data[48:56])[0]
                        change_count = struct.unpack('<H', delta_data[56:58])[0]
                        
                        # Convert timestamp to readable format
                        timestamp_ms = timestamp_ns / 1_000_000
                        age_ms = (time.time() * 1000) - timestamp_ms
                        
                        print(f"   Delta {i+1}: {symbol}@{exchange}")
                        print(f"     Version: {version} (prev: {prev_version})")
                        print(f"     Changes: {change_count}")
                        print(f"     Age: {age_ms:.1f}ms")
                        
                        # Parse first few price level changes
                        changes_offset = 58
                        change_size = 12  # PriceLevelChange size
                        
                        if change_count > 0:
                            print(f"     Price Level Changes:")
                            for j in range(min(3, change_count)):  # Show first 3 changes
                                change_offset = changes_offset + (j * change_size)
                                if change_offset + change_size <= len(delta_data):
                                    price = struct.unpack('<d', delta_data[change_offset:change_offset+8])[0]
                                    volume = struct.unpack('<d', delta_data[change_offset+8:change_offset+16])[0]
                                    side_and_action = delta_data[change_offset+16]
                                    
                                    is_ask = (side_and_action & 0x80) != 0
                                    side = "ask" if is_ask else "bid"
                                    action = "remove" if volume == 0.0 else "update"
                                    
                                    print(f"       {j+1}. {side} ${price:.2f} -> {volume:.6f} ({action})")
                        print()
                
    except Exception as e:
        print(f"❌ Error reading shared memory: {e}")
        return
    
    print("✅ Delta shared memory test complete!")

if __name__ == "__main__":
    test_delta_shared_memory()