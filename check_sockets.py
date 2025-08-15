#!/usr/bin/env python3
import os
import stat
import time

socket_dir = "/tmp/alphapulse"

print(f"Checking socket directory: {socket_dir}")
print("=" * 50)

if not os.path.exists(socket_dir):
    print(f"❌ Directory {socket_dir} does not exist!")
    exit(1)

print(f"✅ Directory {socket_dir} exists")
print(f"Directory permissions: {oct(os.stat(socket_dir).st_mode)}")
print()

files = os.listdir(socket_dir)
if not files:
    print("❌ No files in socket directory")
else:
    print(f"✅ Found {len(files)} file(s):")
    for file in files:
        file_path = os.path.join(socket_dir, file)
        file_stat = os.stat(file_path)
        file_type = "socket" if stat.S_ISSOCK(file_stat.st_mode) else "regular file"
        print(f"  - {file} ({file_type}) - {oct(file_stat.st_mode)}")

print()

# Check specific socket files we expect
expected_sockets = ["coinbase.sock", "kraken.sock", "binance.sock", "relay.sock"]
for socket_name in expected_sockets:
    socket_path = os.path.join(socket_dir, socket_name)
    if os.path.exists(socket_path):
        file_stat = os.stat(socket_path)
        if stat.S_ISSOCK(file_stat.st_mode):
            print(f"✅ {socket_name} - Socket file exists and is a proper socket")
        else:
            print(f"⚠️  {socket_name} - File exists but is not a socket")
    else:
        print(f"❌ {socket_name} - Does not exist")