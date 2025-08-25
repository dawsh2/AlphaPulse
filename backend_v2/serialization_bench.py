#!/usr/bin/env python3
import json
import struct
import time
import csv
from io import StringIO

# Sample market data
market_data = {
    "symbol": "BTCUSD",
    "price": 45000.50,
    "volume": 1.5,
    "timestamp": 1704110400,
    "bid": 44999.00,
    "ask": 45001.00
}

# Simulate 100k market data events
num_events = 100_000

print("🔬 Serialization Overhead Comparison")
print("=" * 50)

# 1. JSON serialization
start = time.perf_counter()
for _ in range(num_events):
    json_bytes = json.dumps(market_data).encode()
    parsed = json.loads(json_bytes.decode())
json_time = time.perf_counter() - start
json_size = len(json_bytes)

print(f"\n📊 JSON:")
print(f"  Time: {json_time:.3f}s ({num_events/json_time:.0f} msg/s)")
print(f"  Size: {json_size} bytes per message")
print(f"  Latency: {json_time/num_events*1_000_000:.1f}μs per message")

# 2. Binary serialization (like TLV)
fmt = 'dddqdd'  # double, double, double, long, double, double
start = time.perf_counter()
for _ in range(num_events):
    binary_bytes = struct.pack(fmt,
        market_data["price"],
        market_data["volume"],
        market_data["bid"],
        market_data["timestamp"],
        market_data["ask"],
        45000.0  # dummy field
    )
    parsed = struct.unpack(fmt, binary_bytes)
binary_time = time.perf_counter() - start
binary_size = len(binary_bytes)

print(f"\n📊 Binary (TLV-like):")
print(f"  Time: {binary_time:.3f}s ({num_events/binary_time:.0f} msg/s)")
print(f"  Size: {binary_size} bytes per message")
print(f"  Latency: {binary_time/num_events*1_000_000:.1f}μs per message")

# 3. CSV parsing simulation
csv_data = "BTCUSD,45000.50,1.5,1704110400,44999.00,45001.00\n" * 1000
start = time.perf_counter()
for _ in range(100):  # 100 batches of 1000 rows
    reader = csv.reader(StringIO(csv_data))
    for row in reader:
        price = float(row[1])
        volume = float(row[2])
csv_time = time.perf_counter() - start
csv_per_row = csv_time / 100_000

print(f"\n📊 CSV Parsing:")
print(f"  Time per row: {csv_per_row*1_000_000:.1f}μs")
print(f"  Throughput: {1/csv_per_row:.0f} rows/s")

# 4. Direct object passing (no serialization)
start = time.perf_counter()
for _ in range(num_events):
    # Just pass the dict reference
    data = market_data
    price = data["price"]
object_time = time.perf_counter() - start

print(f"\n📊 Direct Object (no serialization):")
print(f"  Time: {object_time:.3f}s ({num_events/object_time:.0f} msg/s)")
print(f"  Latency: {object_time/num_events*1_000_000:.3f}μs per message")

print(f"\n🎯 Summary:")
print(f"  JSON is {json_time/binary_time:.1f}x slower than binary")
print(f"  Binary is {binary_time/object_time:.1f}x slower than direct objects")
print(f"  JSON uses {json_size/binary_size:.1f}x more bytes than binary")

print(f"\n💡 Recommendations:")
print(f"  • Same process: Pass objects directly (no serialization)")
print(f"  • Same machine: Use binary/TLV over Unix sockets")
print(f"  • Network: Binary/TLV still wins over JSON")
print(f"  • CSV/Parquet input: Parse once → TLV → distribute")
