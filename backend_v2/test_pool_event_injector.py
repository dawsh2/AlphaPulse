#!/usr/bin/env python3
"""
Test pool event injector for backend_v2 system.

This sends properly formatted TLV messages to the relay to simulate 
what the polygon collector should be doing. This tests the backend_v2 
pipeline without relying on broken WebSocket subscriptions.
"""

import socket
import struct
import time
import json
import random

def create_tlv_header(tlv_type, payload_length):
    """Create a TLV header (simple version for testing)"""
    # Magic: 0xDEADBEEF (4 bytes)
    # TLV Type: 1 byte  
    # Length: 2 bytes
    # Padding: 1 byte
    return struct.pack('<I B H B', 0xDEADBEEF, tlv_type, payload_length, 0)

def create_fake_pool_swap():
    """Create fake pool swap data"""
    return {
        'venue_name': 'Polygon',
        'pool_address': '0x45dda9cb7c25131df268515131f647d726f50608',  # USDC/WETH pool
        'token0_symbol': 'USDC',
        'token1_symbol': 'WETH', 
        'amount0_delta': random.randint(-2000000, -100000),  # -2 to -0.1 USDC (6 decimals)
        'amount1_delta': random.randint(100000000000000, 1000000000000000),  # 0.0001 to 0.001 WETH (18 decimals)
        'sqrt_price_x96': 79228162514264337593543950336,  # ~$2000 per ETH
        'liquidity': random.randint(1000000000000, 10000000000000),
        'tick': random.randint(-276000, -275000),
        'fee_paid': 500,  # 0.5 USDC
        'timestamp': int(time.time() * 1000000000),  # nanoseconds
        'block_number': 12345678 + random.randint(0, 1000),
        'log_index': random.randint(1, 100)
    }

def inject_pool_events():
    """Inject fake pool events into the Unix socket relay"""
    print("🧪 Backend V2 Pool Event Injector")
    print("📡 Connecting to relay...")
    
    try:
        # Connect to the market data relay Unix socket
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect("/tmp/alphapulse/market_data.sock")
        print("✅ Connected to market data relay")
        
        event_count = 0
        
        while event_count < 20:  # Send 20 events
            # Create fake pool swap
            pool_swap = create_fake_pool_swap()
            
            # For testing, send as JSON (real TLV would be binary)
            # The dashboard converter should handle this
            message_data = json.dumps({
                "msg_type": "pool_swap",
                **pool_swap
            }).encode('utf-8')
            
            # Simple framing: length + message
            length_header = struct.pack('<I', len(message_data))
            
            try:
                sock.send(length_header + message_data)
                event_count += 1
                print(f"📤 Sent pool event #{event_count}: {pool_swap['amount0_delta']/1000000:.2f} USDC → {pool_swap['amount1_delta']/1000000000000000000:.4f} WETH")
                
                time.sleep(2)  # Send every 2 seconds
                
            except Exception as e:
                print(f"❌ Failed to send event: {e}")
                break
                
    except Exception as e:
        print(f"❌ Connection error: {e}")
        print("   Make sure the relay is running")
    finally:
        try:
            sock.close()
        except:
            pass
    
    print(f"✅ Sent {event_count} pool events to backend_v2 relay")
    print("📱 Check frontend at http://localhost:5177 for pool events")

if __name__ == "__main__":
    inject_pool_events()