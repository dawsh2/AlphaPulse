#!/usr/bin/env python3
"""Mock relay server that accepts Unix socket connections."""

import asyncio
import os
import socket
import struct

SOCKET_PATH = "/tmp/alphapulse/market_data.sock"

async def handle_client(reader, writer):
    """Handle a client connection."""
    client_addr = writer.get_extra_info('peername')
    print(f"✅ Client connected: {client_addr}")
    
    try:
        while True:
            # Read data from client
            data = await reader.read(1024)
            if not data:
                break
            
            # Just acknowledge receipt (mock behavior)
            print(f"📨 Received {len(data)} bytes")
            
    except Exception as e:
        print(f"❌ Client error: {e}")
    finally:
        writer.close()
        await writer.wait_closed()
        print(f"👋 Client disconnected: {client_addr}")

async def main():
    """Start the mock relay server."""
    # Create directory if it doesn't exist
    os.makedirs("/tmp/alphapulse", exist_ok=True)
    
    # Remove existing socket if it exists
    if os.path.exists(SOCKET_PATH):
        os.unlink(SOCKET_PATH)
    
    # Create Unix socket server
    server = await asyncio.start_unix_server(handle_client, SOCKET_PATH)
    
    print(f"🚀 Mock MarketDataRelay started on {SOCKET_PATH}")
    print("📡 Waiting for connections...")
    
    async with server:
        await server.serve_forever()

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n👋 Shutting down mock relay")