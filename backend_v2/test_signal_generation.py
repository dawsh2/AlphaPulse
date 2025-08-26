#!/usr/bin/env python3
"""
Test script to generate mock arbitrage signals and verify they flow through to dashboard
"""

import socket
import struct
import time
from typing import Dict, Any


def create_demo_defi_arbitrage_tlv() -> bytes:
    """
    Create a DemoDeFiArbitrageTLV message (Extended TLV type 255)
    Struct layout matches Rust DemoDeFiArbitrageTLV (226 bytes)
    """
    # ArbitrageConfig fields (226 bytes total)
    strategy_id = 21  # Flash arbitrage (u16)
    signal_id = int(time.time() * 1000000)  # Use microsecond timestamp (u64)
    confidence = 85  # 85% confidence (u8)
    chain_id = 137  # Polygon (u32)
    expected_profit_q = int(125.75 * 100000000)  # $125.75 in 8 decimal fixed point (i128)
    required_capital_q = int(5000.0 * 100000000)  # $5000 in 8 decimal fixed point (u128)
    estimated_gas_cost_q = int(2.50 * 100000000)  # $2.50 gas cost (u128)
    
    # Venue IDs (u16 each)
    venue_a = 103  # QuickSwap
    venue_b = 104  # SushiSwap Polygon
    
    # Pool addresses (32 bytes each, zero-padded 20-byte addresses)
    pool_a = b'\x00' * 12 + bytes.fromhex('1f98431c8ad98523631ae4a59f267346ea31f984')  # Real Uniswap V3 factory
    pool_b = b'\x00' * 12 + bytes.fromhex('c35dadb65012ec5796536bd9864ed8773abc74c4')  # Real SushiSwap factory
    
    # Token addresses (u64 each - truncated for space)
    token_in = 0x2791bca1f2de4661  # USDC on Polygon (truncated)
    token_out = 0x0d500b1d8e8ef31e  # WMATIC on Polygon (truncated)
    
    optimal_amount_q = int(5000.0 * 100000000)  # Same as capital (u128)
    slippage_tolerance = 100  # 1% in basis points (u16)
    max_gas_price_gwei = 20  # 20 gwei (u16)
    valid_until = int(time.time()) + 300  # 5 minutes from now (u32)
    priority = 1  # High priority (u8)
    timestamp_ns = int(time.time() * 1_000_000_000)  # Current time in nanoseconds (u64)
    
    # Pack the struct using little-endian format
    message_data = struct.pack(
        '<'  # Little-endian
        'H'  # strategy_id (u16)
        'Q'  # signal_id (u64)
        'B'  # confidence (u8)
        'I'  # chain_id (u32)
        'q'  # expected_profit_q (i128 - use q for i64, will pad manually)
        '8x'  # Padding for i128 (extra 8 bytes)
        'Q'  # required_capital_q (u128 - use Q for u64, will pad manually)
        '8x'  # Padding for u128 (extra 8 bytes)
        'Q'  # estimated_gas_cost_q (u128 - use Q for u64, will pad manually)  
        '8x'  # Padding for u128 (extra 8 bytes)
        'H'  # venue_a (u16)
        '32s'  # pool_a (32 bytes)
        'H'  # venue_b (u16)
        '32s'  # pool_b (32 bytes)
        'Q'  # token_in (u64)
        'Q'  # token_out (u64)
        'Q'  # optimal_amount_q (u128 - use Q for u64, will pad manually)
        '8x'  # Padding for u128 (extra 8 bytes)
        'H'  # slippage_tolerance (u16)
        'H'  # max_gas_price_gwei (u16)
        'I'  # valid_until (u32)
        'B'  # priority (u8)
        'Q',  # timestamp_ns (u64)
        strategy_id,
        signal_id,
        confidence,
        chain_id,
        expected_profit_q,
        required_capital_q,
        estimated_gas_cost_q,
        venue_a,
        pool_a,
        venue_b,
        pool_b,
        token_in,
        token_out,
        optimal_amount_q,
        slippage_tolerance,
        max_gas_price_gwei,
        valid_until,
        priority,
        timestamp_ns
    )
    
    return message_data


def create_tlv_message(tlv_type: int, tlv_data: bytes) -> bytes:
    """
    Create Protocol V2 TLV message with 32-byte header
    """
    # Calculate payload size
    payload_size = len(tlv_data) + 6  # Extended TLV header (6 bytes) + data
    
    # Create TLV payload (Extended TLV format)
    extended_tlv_header = struct.pack(
        '<HHH',  # Little-endian: tlv_type (u16), reserved (u16), tlv_length (u16)
        tlv_type,
        0,  # reserved
        len(tlv_data)
    )
    tlv_payload = extended_tlv_header + tlv_data
    
    # Create 32-byte message header
    magic = 0xDEADBEEF
    relay_domain = 1  # Signal domain
    source = 2  # ArbitrageStrategy
    sequence = int(time.time()) & 0xFFFFFFFF
    timestamp = int(time.time() * 1_000_000_000)  # nanoseconds
    checksum = 0  # Not used for signal domain
    version = 1
    flags = 0
    
    header = struct.pack(
        '<IBBIIIHBB',  # Little-endian
        magic,
        relay_domain,
        source,
        sequence,
        timestamp,
        payload_size,
        checksum,
        version,
        flags
    )
    
    # Pad header to exactly 32 bytes
    header = header.ljust(32, b'\x00')
    
    return header + tlv_payload


def send_signal_to_relay(message: bytes, socket_path: str = "/tmp/alphapulse/signals.sock"):
    """
    Send signal message to the signal relay
    """
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect(socket_path)
        
        print(f"📡 Sending {len(message)} bytes to signal relay...")
        print(f"Message preview: {message[:64].hex()}")
        
        sock.send(message)
        sock.close()
        
        print("✅ Signal sent successfully!")
        return True
        
    except Exception as e:
        print(f"❌ Failed to send signal: {e}")
        return False


def main():
    print("🧪 Testing arbitrage signal generation and relay...")
    
    # Create demo arbitrage signal
    arbitrage_data = create_demo_defi_arbitrage_tlv()
    print(f"📦 Created DemoDeFiArbitrageTLV: {len(arbitrage_data)} bytes")
    
    # Wrap in TLV message
    tlv_message = create_tlv_message(255, arbitrage_data)  # Extended TLV type
    print(f"📬 Created complete TLV message: {len(tlv_message)} bytes")
    
    # Send to signal relay
    success = send_signal_to_relay(tlv_message)
    
    if success:
        print("🎯 Signal should now flow to dashboard via Signal Relay!")
        print("Check the dashboard at http://localhost:3001 for the arbitrage opportunity")
    else:
        print("💥 Signal sending failed - check if signal relay is running")


if __name__ == "__main__":
    main()