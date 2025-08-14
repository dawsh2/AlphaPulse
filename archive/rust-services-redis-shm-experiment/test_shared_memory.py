#!/usr/bin/env python3
"""Test script to verify shared memory implementation is working"""
import time
import struct
import mmap
import os

def read_shared_memory():
    path = "/tmp/alphapulse_shm/trades"
    
    if not os.path.exists(path):
        print(f"❌ Shared memory file not found at {path}")
        return
    
    file_size = os.path.getsize(path)
    print(f"✅ Shared memory file exists: {path}")
    print(f"   File size: {file_size:,} bytes")
    
    with open(path, "r+b") as f:
        # Memory map the file
        mm = mmap.mmap(f.fileno(), 0)
        
        # Calculate header size
        header_size = 4 + 4 + 8 + 8 + 4 + 8 + (8 * 16) + 64  # = 232 bytes
        header_data = mm[:header_size]
        
        # Parse header fields
        version, capacity = struct.unpack("II", header_data[0:8])
        write_sequence = struct.unpack("Q", header_data[8:16])[0]
        
        print(f"\n📊 Shared Memory Stats:")
        print(f"   Version: {version}")
        print(f"   Capacity: {capacity:,} trades")
        print(f"   Write sequence: {write_sequence:,} trades written")
        
        # Calculate trades per second
        if write_sequence > 0:
            # Each trade is 128 bytes
            trades_offset = header_size
            
            # Read first trade
            first_trade = mm[trades_offset:trades_offset+128]
            first_timestamp_ns = struct.unpack("Q", first_trade[0:8])[0]
            
            # Read last trade
            last_index = ((write_sequence - 1) % capacity)
            last_trade_offset = trades_offset + (last_index * 128)
            last_trade = mm[last_trade_offset:last_trade_offset+128]
            last_timestamp_ns = struct.unpack("Q", last_trade[0:8])[0]
            
            # Parse trade details
            symbol = last_trade[8:24].decode('utf-8', errors='ignore').rstrip('\x00')
            exchange = last_trade[24:40].decode('utf-8', errors='ignore').rstrip('\x00')
            price = struct.unpack("d", last_trade[40:48])[0]
            volume = struct.unpack("d", last_trade[48:56])[0]
            side = last_trade[56]
            
            print(f"\n📈 Last Trade:")
            print(f"   Symbol: {symbol}")
            print(f"   Exchange: {exchange}")
            print(f"   Price: ${price:,.2f}")
            print(f"   Volume: {volume:.4f}")
            print(f"   Side: {'buy' if side == 0 else 'sell'}")
            
            # Calculate throughput
            if last_timestamp_ns > first_timestamp_ns:
                duration_seconds = (last_timestamp_ns - first_timestamp_ns) / 1_000_000_000
                trades_per_sec = write_sequence / duration_seconds
                print(f"\n⚡ Performance:")
                print(f"   Duration: {duration_seconds:.1f} seconds")
                print(f"   Throughput: {trades_per_sec:.1f} trades/sec")
        
        mm.close()

if __name__ == "__main__":
    print("🔍 Testing AlphaPulse Shared Memory Implementation\n")
    read_shared_memory()