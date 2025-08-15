#!/usr/bin/env python3
import socket
import os
import time

# Ensure directory exists
os.makedirs("/tmp/alphapulse", exist_ok=True)

socket_path = "/tmp/alphapulse/test.sock"

# Remove socket if it exists
try:
    os.unlink(socket_path)
except FileNotFoundError:
    pass

# Create server socket
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(socket_path)
server.listen(1)

print(f"Server listening on {socket_path}")
print(f"Socket file exists: {os.path.exists(socket_path)}")
print(f"Socket file permissions: {oct(os.stat(socket_path).st_mode)}")

# Try to connect as client
try:
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.connect(socket_path)
    print("Client connection successful!")
    client.close()
except Exception as e:
    print(f"Client connection failed: {e}")

server.close()
os.unlink(socket_path)
print("Test completed")