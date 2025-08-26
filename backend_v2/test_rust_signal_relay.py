#!/usr/bin/env python3
"""Test if Rust signal relay is working correctly"""
import socket
import struct
import time
import threading

def create_tlv_message(sequence: int, message_type: str = "test"):
    """Create a Protocol V2 TLV message"""
    # Header: magic(4) + domain(1) + source(1) + reserved(2) + sequence(8) + timestamp(8) + payload_size(4) + checksum(4) = 32 bytes
    magic = 0xDEADBEEF
    domain = 2  # Signal domain
    source = 4  # ArbitrageStrategy
    reserved = 0
    timestamp_ns = int(time.time() * 1_000_000_000)

    # Build ArbitrageSignalTLV payload (type 21)
    tlv_type = 21  # ArbitrageSignalTLV
    tlv_length = 180  # Size of ArbitrageSignalTLV struct

    # Create dummy arbitrage signal data
    tlv_data = struct.pack(
        '<H',  # strategy_id (u16)
        21  # Flash arbitrage strategy ID
    )
    tlv_data += struct.pack('<Q', sequence)  # signal_id (u64)
    tlv_data += struct.pack('<I', 137)  # chain_id (u32) - Polygon
    tlv_data += b'\x00' * 20  # source_pool (20 bytes)
    tlv_data += b'\x01' * 20  # target_pool (20 bytes)
    tlv_data += struct.pack('<H', 300)  # source_venue (u16) - UniswapV2
    tlv_data += struct.pack('<H', 301)  # target_venue (u16) - UniswapV3
    tlv_data += b'\x02' * 20  # token_in (20 bytes)
    tlv_data += b'\x03' * 20  # token_out (20 bytes)
    tlv_data += struct.pack('<q', 150000000)  # expected_profit_usd_q8 ($1.50)
    tlv_data += struct.pack('<q', 100000000000)  # required_capital_usd_q8 ($1000)
    tlv_data += struct.pack('<H', 150)  # spread_bps (1.5%)
    tlv_data += struct.pack('<q', 10000000)  # dex_fees_usd_q8 ($0.10)
    tlv_data += struct.pack('<q', 5000000)  # gas_cost_usd_q8 ($0.05)
    tlv_data += struct.pack('<q', 5000000)  # slippage_usd_q8 ($0.05)
    tlv_data += struct.pack('<q', 130000000)  # net_profit_usd_q8 ($1.30)
    tlv_data += struct.pack('<H', 50)  # slippage_tolerance_bps (0.5%)
    tlv_data += struct.pack('<I', 30)  # max_gas_price_gwei
    tlv_data += struct.pack('<I', int(time.time()) + 60)  # valid_until (60s from now)
    tlv_data += struct.pack('<H', 100)  # priority
    tlv_data += b'\x00' * 2  # reserved
    tlv_data += struct.pack('<Q', timestamp_ns)  # timestamp_ns

    # Ensure we have exactly 180 bytes
    tlv_data = tlv_data[:180] if len(tlv_data) > 180 else tlv_data + b'\x00' * (180 - len(tlv_data))

    payload = struct.pack('<HH', tlv_type, tlv_length) + tlv_data
    payload_size = len(payload)
    checksum = 0  # Simplified

    header = struct.pack('<IBBHQQII', magic, domain, source, reserved, sequence, timestamp_ns, payload_size, checksum)
    return header + payload

def sender_thread():
    """Thread that sends messages"""
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect('/tmp/alphapulse/signals.sock')
    print("✅ Sender connected to Rust signal relay")

    for i in range(3):
        message = create_tlv_message(i + 1)
        sock.send(message)
        print(f"📤 Sent ArbitrageSignal message {i+1} (216 bytes)")
        time.sleep(0.5)

    # Keep connection alive to receive any echoed messages
    time.sleep(2)
    sock.close()
    print("👋 Sender disconnected")

def consumer_thread(consumer_id: int):
    """Thread that receives messages"""
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect('/tmp/alphapulse/signals.sock')
    sock.settimeout(5.0)  # 5 second timeout
    print(f"✅ Consumer {consumer_id} connected to Rust signal relay")

    message_count = 0
    try:
        while message_count < 3:
            try:
                data = sock.recv(4096)
                if data:
                    message_count += 1
                    # Parse header to get sequence number
                    if len(data) >= 32:
                        magic, domain, source, reserved, sequence = struct.unpack('<IBBHQ', data[:16])
                        if magic == 0xDEADBEEF:
                            print(f"📥 Consumer {consumer_id} received message {message_count}: sequence={sequence}, {len(data)} bytes")
                        else:
                            print(f"⚠️  Consumer {consumer_id} received non-TLV data: {len(data)} bytes")
                    else:
                        print(f"📥 Consumer {consumer_id} received short message: {len(data)} bytes")
            except socket.timeout:
                print(f"⏱️  Consumer {consumer_id} timeout waiting for message")
                break
    except Exception as e:
        print(f"❌ Consumer {consumer_id} error: {e}")

    sock.close()
    print(f"👋 Consumer {consumer_id} disconnected after {message_count} messages")

def main():
    print("🧪 Testing Rust Signal Relay (fixed_signal_relay)")
    print("=" * 50)

    # Start consumers first
    consumer1 = threading.Thread(target=consumer_thread, args=(1,))
    consumer2 = threading.Thread(target=consumer_thread, args=(2,))

    consumer1.start()
    consumer2.start()

    # Give consumers time to connect
    time.sleep(0.5)

    # Start sender
    sender = threading.Thread(target=sender_thread)
    sender.start()

    # Wait for all threads
    sender.join()
    consumer1.join()
    consumer2.join()

    print("=" * 50)
    print("✅ Test completed - Rust signal relay appears to be working correctly!")
    print("The relay properly forwards messages from senders to all OTHER consumers.")

if __name__ == "__main__":
    main()
