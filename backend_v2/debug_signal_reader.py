#!/usr/bin/env python3
"""
Debug tool to read and display Signal relay messages
"""
import socket
import struct
import time

SIGNAL_SOCKET = '/tmp/alphapulse/signals.sock'

def parse_tlv_header(data):
    """Parse Protocol V2 message header (32 bytes)"""
    if len(data) < 32:
        return None

    # Parse header: magic(4) + domain(1) + source(1) + reserved(2) + sequence(8) + timestamp(8) + payload_size(4) + checksum(4)
    magic, domain, source = struct.unpack('<I B B', data[:6])
    sequence = struct.unpack('<Q', data[8:16])[0]
    timestamp_ns = struct.unpack('<Q', data[16:24])[0]
    payload_size = struct.unpack('<I', data[24:28])[0]

    return {
        'magic': f'0x{magic:08X}',
        'domain': domain,
        'source': source,
        'sequence': sequence,
        'timestamp_ns': timestamp_ns,
        'payload_size': payload_size
    }

def parse_tlv_payload(data):
    """Parse TLV payload"""
    offset = 0
    tlvs = []

    while offset + 4 <= len(data):
        # TLV header: type(2) + length(2)
        tlv_type, tlv_length = struct.unpack('<HH', data[offset:offset+4])
        offset += 4

        if offset + tlv_length > len(data):
            break

        tlv_data = data[offset:offset+tlv_length]
        offset += tlv_length

        # Show first 32 bytes of TLV data as hex
        hex_preview = ' '.join(f'{b:02x}' for b in tlv_data[:min(32, len(tlv_data))])
        if len(tlv_data) > 32:
            hex_preview += '...'

        tlvs.append({
            'type': tlv_type,
            'length': tlv_length,
            'data_preview': hex_preview
        })

    return tlvs

def main():
    print(f"Connecting to Signal relay at {SIGNAL_SOCKET}...")

    # Connect to Unix socket
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(SIGNAL_SOCKET)
    print("Connected! Reading messages...")

    buffer = b''
    message_count = 0

    while True:
        # Read data
        data = sock.recv(4096)
        if not data:
            print("Connection closed")
            break

        buffer += data

        # Process complete messages
        while len(buffer) >= 32:
            header = parse_tlv_header(buffer[:32])
            if not header or header['magic'] != '0xDEADBEEF':
                print(f"Invalid magic: {header['magic'] if header else 'None'}")
                buffer = buffer[1:]  # Skip one byte and try again
                continue

            total_size = 32 + header['payload_size']
            if len(buffer) < total_size:
                break  # Wait for more data

            # Extract complete message
            message = buffer[:total_size]
            buffer = buffer[total_size:]

            message_count += 1

            # Parse payload
            payload = message[32:]
            tlvs = parse_tlv_payload(payload)

            # Display message info
            timestamp_sec = header['timestamp_ns'] / 1_000_000_000
            print(f"\n--- Message #{message_count} ---")
            print(f"Time: {time.strftime('%H:%M:%S', time.localtime(timestamp_sec))}")
            print(f"Domain: {header['domain']} | Source: {header['source']} | Seq: {header['sequence']}")
            print(f"Payload: {header['payload_size']} bytes")

            for tlv in tlvs:
                print(f"  TLV Type {tlv['type']:3d} | Len {tlv['length']:4d} | {tlv['data_preview']}")

                # Special handling for type 255 (DemoDeFiArbitrageTLV)
                if tlv['type'] == 255:
                    print(f"  ^^ This is a DemoDeFiArbitrageTLV arbitrage signal!")

if __name__ == '__main__':
    main()
