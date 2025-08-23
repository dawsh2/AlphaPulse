#!/usr/bin/env python3
"""
Test script to send a sample ArbitrageOpportunityMessage to relay for dashboard testing
"""

import socket
import struct
import time

def create_arbitrage_opportunity_message():
    """Create a test ArbitrageOpportunityMessage (208 bytes)"""
    
    # Message header (32 bytes)
    magic = 0xDEADBEEF  # 4 bytes
    msg_type = 0x06  # ArbitrageOpportunity type (1 byte)
    payload_size = 176  # 208 - 32 = 176 bytes payload (2 bytes)
    reserved = 0  # 1 byte
    timestamp = int(time.time() * 1_000_000_000)  # 8 bytes - nanoseconds
    source_id = 3  # Scanner source (2 bytes)
    sequence = 1  # 4 bytes
    exchange_id = 137  # Polygon (2 bytes)
    extra_flags = 0  # 8 bytes
    
    # Header = 32 bytes
    header = struct.pack('<LHBBQHLL', magic, msg_type, payload_size, reserved, timestamp, source_id, sequence, exchange_id) + b'\x00' * 8
    
    # ArbitrageOpportunity payload (176 bytes)
    # Token and pool IDs (48 bytes = 4 * 12)
    token0_id = b'\x89\x00' + b'\x01' + b'\x00' + (1234567890).to_bytes(8, 'little')  # 12 bytes
    token1_id = b'\x89\x00' + b'\x01' + b'\x00' + (1234567891).to_bytes(8, 'little')  # 12 bytes
    buy_pool_id = b'\x89\x00' + b'\x02' + b'\x00' + (5555555555).to_bytes(8, 'little')  # 12 bytes
    sell_pool_id = b'\x89\x00' + b'\x02' + b'\x00' + (6666666666).to_bytes(8, 'little')  # 12 bytes
    
    # Price and trading data (48 bytes = 6 * 8)
    buy_price = int(1.5 * 100_000_000)  # $1.50 in 8 decimal fixed point
    sell_price = int(1.52 * 100_000_000)  # $1.52 in 8 decimal fixed point
    trade_size_usd = int(1000 * 100_000_000)  # $1000
    gross_profit_usd = int(20 * 100_000_000)  # $20 gross profit
    gas_fee_usd = int(2.5 * 100_000_000)  # $2.50 gas
    dex_fees_usd = int(3.0 * 100_000_000)  # $3.00 DEX fees
    
    # More trading data (16 bytes = 2 * 8)
    slippage_cost_usd = int(0.5 * 100_000_000)  # $0.50 slippage
    net_profit_usd = int(14.0 * 100_000_000)  # $14.00 net profit
    
    # Percentage and flags (8 bytes)
    profit_percent = int(1.4 * 10000)  # 1.4% in 4 decimal precision
    confidence_score = int(0.95 * 1000)  # 95% confidence
    executable = 1  # executable
    padding = 0
    
    # Token symbols and exchange names (64 bytes = 4 * 16)
    token0_symbol = b'WETH\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00'  # 16 bytes
    token1_symbol = b'USDC\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00'  # 16 bytes
    buy_exchange = b'UniswapV2\x00\x00\x00\x00\x00\x00\x00'  # 16 bytes
    sell_exchange = b'SushiSwap\x00\x00\x00\x00\x00\x00\x00'  # 16 bytes
    
    # Assemble payload (176 bytes total)
    payload = (token0_id + token1_id + buy_pool_id + sell_pool_id +
               struct.pack('<QQQQQQQQ', buy_price, sell_price, trade_size_usd, gross_profit_usd, 
                          gas_fee_usd, dex_fees_usd, slippage_cost_usd, net_profit_usd) +
               struct.pack('<LHBB', profit_percent, confidence_score, executable, padding) +
               token0_symbol + token1_symbol + buy_exchange + sell_exchange)
    
    # Verify sizes
    assert len(header) == 32, f"Header size: {len(header)}"
    assert len(payload) == 176, f"Payload size: {len(payload)}"
    
    message = header + payload
    assert len(message) == 208, f"Total message size: {len(message)}"
    
    return message

def send_to_relay():
    """Send test arbitrage message to relay server"""
    try:
        # Connect to relay server as a "scanner"
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect('/tmp/alphapulse/polygon.sock')  # Connect as polygon exchange
        
        # Create and send test message
        message = create_arbitrage_opportunity_message()
        print(f"Sending {len(message)} byte ArbitrageOpportunityMessage to relay...")
        
        # Send the message
        sock.sendall(message)
        print("✅ Message sent successfully!")
        
        # Keep connection open briefly
        time.sleep(1)
        
    except Exception as e:
        print(f"❌ Error sending message: {e}")
    finally:
        try:
            sock.close()
        except:
            pass

if __name__ == "__main__":
    print("🚀 Testing ArbitrageOpportunityMessage flow to dashboard...")
    send_to_relay()
    print("✅ Test completed. Check ws-bridge and dashboard logs.")