#!/usr/bin/env python3
"""Test consumer for Signal relay - verifies messages are being forwarded"""
import socket
import time

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect('/tmp/alphapulse/signals.sock')
print("Connected to signal relay as consumer")

# Just read and print any data received
total_bytes = 0
message_count = 0
while True:
    data = sock.recv(4096)
    if data:
        total_bytes += len(data)
        message_count += 1
        # Check for TLV type 255 (0xFF in the payload after header)
        if b'\xff\x00' in data or b'\x00\xff' in data:
            print(f"🎯 Found TLV type 255 message! Total: {message_count} messages, {total_bytes} bytes")
        if message_count % 10 == 0:
            print(f"Received {message_count} messages, {total_bytes} bytes total")
    else:
        print("Connection closed")
        break
