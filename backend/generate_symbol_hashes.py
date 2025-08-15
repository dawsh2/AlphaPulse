#!/usr/bin/env python3
"""Generate symbol hashes matching Rust's DefaultHasher for frontend"""

import struct

def rust_default_hasher(s: str) -> int:
    """
    Approximation of Rust's DefaultHasher (SipHash 1-3)
    This is a simplified version - for exact hashes we need to match
    the binary protocol implementation
    """
    # For now, use Python's hash - we'll get exact values from the actual system
    # The important thing is consistency between backend and frontend
    import hashlib
    h = hashlib.sha256(s.encode()).digest()
    # Take first 8 bytes as u64
    return struct.unpack('<Q', h[:8])[0]

# These are the actual hashes from the logs we've seen
KNOWN_HASHES = {
    'coinbase:BTC-USD': 16842681295735137662,
    'coinbase:ETH-USD': 7334401999635196894,
}

# Additional symbols we want to support
SYMBOLS = [
    ('coinbase:BTC-USD', 'BTC-USD'),
    ('coinbase:ETH-USD', 'ETH-USD'),
    ('coinbase:SOL-USD', 'SOL-USD'),
    ('coinbase:LINK-USD', 'LINK-USD'),
    ('coinbase:AVAX-USD', 'AVAX-USD'),
    ('coinbase:MATIC-USD', 'MATIC-USD'),
    ('coinbase:ADA-USD', 'ADA-USD'),
    ('coinbase:DOT-USD', 'DOT-USD'),
    ('coinbase:BTC-USDT', 'BTC-USDT'),
    ('coinbase:ETH-USDT', 'ETH-USDT'),
    ('alpaca:AAPL', 'AAPL'),
    ('alpaca:GOOGL', 'GOOGL'),
    ('alpaca:MSFT', 'MSFT'),
    ('alpaca:TSLA', 'TSLA'),
    ('alpaca:NVDA', 'NVDA'),
    ('alpaca:META', 'META'),
    ('alpaca:AMD', 'AMD'),
    ('alpaca:SPY', 'SPY'),
    ('alpaca:QQQ', 'QQQ'),
    ('alpaca:AMZN', 'AMZN'),
]

print("// Generated symbol hash mappings")
print("// Copy these to frontend/src/dashboard/utils/symbolHash.ts")
print()
print("const HASH_TO_SYMBOL: Record<string, string> = {")

# First, add the known hashes
for canonical, hash_val in KNOWN_HASHES.items():
    display = canonical.split(':')[1]
    print(f"  '{hash_val}': '{display}', // {canonical}")

# For other symbols, we'll use placeholder values until we get the actual hashes
for canonical, display in SYMBOLS:
    if canonical not in KNOWN_HASHES:
        # Use a placeholder that makes it clear these need real values
        print(f"  // '{canonical}': '{display}', // TODO: Get actual hash from backend")

print("};")
print()
print("// To get actual hashes, run the exchange collector and check the logs")
print("// or use the debug_symbols.py script to query the running system")