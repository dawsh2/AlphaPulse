#!/usr/bin/env python3
"""Test script to directly read from the market data relay socket."""

import socket
import struct
import time

def connect_to_relay():
    """Connect to the market data relay unix socket."""
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect('/tmp/alphapulse/market_data.sock')
    print("✅ Connected to relay")
    return sock

def read_messages(sock):
    """Read messages from the relay."""
    buffer = b''
    messages_received = 0
    
    while True:
        try:
            # Read data
            data = sock.recv(4096)
            if not data:
                print("❌ Connection closed")
                break
                
            buffer += data
            
            # Process complete messages
            while len(buffer) >= 32:
                # Try to parse header
                magic = struct.unpack('<I', buffer[:4])[0]
                
                if magic == 0xDEADBEEF:
                    # Valid header, get payload size
                    payload_size = struct.unpack('<I', buffer[4:8])[0]
                    total_size = 32 + payload_size
                    
                    if len(buffer) >= total_size:
                        # Complete message
                        message = buffer[:total_size]
                        buffer = buffer[total_size:]
                        messages_received += 1
                        
                        print(f"📨 Message {messages_received}: {total_size} bytes")
                        print(f"   Header preview: {message[:32].hex()}")
                        
                        if messages_received >= 5:
                            print(f"✅ Successfully received {messages_received} messages!")
                            return
                    else:
                        # Wait for more data
                        break
                else:
                    # Check if we have 8 zero bytes (the bug)
                    if buffer[:8] == b'\x00' * 8:
                        print("⚠️  Found 8-byte zero prefix bug!")
                        print(f"   Buffer: {buffer[:16].hex()}")
                        # Skip the zeros
                        buffer = buffer[8:]
                    else:
                        print(f"❌ Invalid magic: 0x{magic:08x}")
                        print(f"   Buffer: {buffer[:16].hex()}")
                        # Skip one byte and try again
                        buffer = buffer[1:]
                        
        except Exception as e:
            print(f"❌ Error: {e}")
            break
    
    print(f"📊 Total messages received: {messages_received}")

if __name__ == "__main__":
    print("🔍 Testing direct relay connection...")
    
    try:
        sock = connect_to_relay()
        read_messages(sock)
        sock.close()
    except Exception as e:
        print(f"❌ Failed: {e}")