#!/usr/bin/env python3
"""Send test messages to Signal relay"""
import socket
import struct
import time

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect('/tmp/alphapulse/signals.sock')
print("Connected to signal relay as sender")

# Send a simple test message with Protocol V2 format
# Header: magic(4) + domain(1) + source(1) + reserved(2) + sequence(8) + timestamp(8) + payload_size(4) + checksum(4) = 32 bytes
for i in range(5):
    # Build header
    magic = 0xDEADBEEF
    domain = 2  # Signal domain
    source = 4  # ArbitrageStrategy
    reserved = 0
    sequence = i
    timestamp_ns = int(time.time() * 1_000_000_000)

    # Build a simple TLV payload with type 255 (DemoDeFiArbitrageTLV)
    tlv_type = 255
    tlv_length = 100  # Some dummy length
    tlv_data = b'TEST_ARBITRAGE_SIGNAL' + b'\x00' * (100 - 21)

    payload = struct.pack('<HH', tlv_type, tlv_length) + tlv_data
    payload_size = len(payload)
    checksum = 0  # Simplified

    header = struct.pack('<IBBHQQII', magic, domain, source, reserved, sequence, timestamp_ns, payload_size, checksum)
    message = header + payload

    sock.send(message)
    print(f"Sent test message {i+1} with TLV type 255")
    time.sleep(1)

print("Done sending test messages")
sock.close()
