"""
Shared utility functions

Pure functions that multiple services can use.
No side effects, no external dependencies.
"""

import re
from typing import Optional, List


def validate_symbol(symbol: str) -> bool:
    """
    Validate trading symbol format
    
    Valid formats:
    - BTC-USD (Coinbase style)
    - BTC/USD (Kraken style)
    - BTCUSD (Binance style)
    """
    pattern = r'^[A-Z]{2,10}[-/]?[A-Z]{2,10}$'
    return bool(re.match(pattern, symbol.upper()))


def normalize_symbol(symbol: str, exchange: str) -> str:
    """
    Normalize symbol to exchange-specific format
    
    Args:
        symbol: Input symbol (BTC-USD, BTC/USD, BTCUSD)
        exchange: Target exchange (coinbase, kraken, binance_us)
    
    Returns:
        Normalized symbol for the exchange
    """
    # Remove any separators and uppercase
    base_symbol = symbol.upper().replace('-', '').replace('/', '')
    
    # Extract base and quote
    # Common pairs - extend as needed
    pairs = [
        ('BTC', 'USD'), ('ETH', 'USD'), ('SOL', 'USD'),
        ('BTC', 'USDT'), ('ETH', 'USDT'), ('SOL', 'USDT'),
    ]
    
    base = None
    quote = None
    for b, q in pairs:
        if base_symbol == b + q:
            base, quote = b, q
            break
    
    if not base:
        # Fallback: assume last 3-4 chars are quote
        if base_symbol.endswith('USDT'):
            quote = 'USDT'
            base = base_symbol[:-4]
        elif base_symbol.endswith('USD'):
            quote = 'USD'
            base = base_symbol[:-3]
        else:
            return symbol  # Can't normalize
    
    # Format for exchange
    if exchange == 'coinbase':
        return f"{base}-{quote}"
    elif exchange == 'kraken':
        return f"{base}/{quote}"
    elif exchange == 'binance_us':
        return f"{base}{quote}"
    else:
        return symbol


def calculate_vwap(trades: List[dict]) -> Optional[float]:
    """
    Calculate Volume-Weighted Average Price
    
    Args:
        trades: List of trade dicts with 'price' and 'volume' keys
    
    Returns:
        VWAP or None if no trades
    """
    if not trades:
        return None
    
    total_value = sum(t['price'] * t['volume'] for t in trades)
    total_volume = sum(t['volume'] for t in trades)
    
    if total_volume == 0:
        return None
    
    return total_value / total_volume


def format_price(price: float, decimals: int = 2) -> str:
    """
    Format price for display
    
    Args:
        price: Price value
        decimals: Number of decimal places
    
    Returns:
        Formatted price string
    """
    if price >= 10000:
        # No decimals for large prices
        return f"{price:,.0f}"
    elif price >= 100:
        # 2 decimals for medium prices
        return f"{price:,.2f}"
    elif price >= 1:
        # 4 decimals for small prices
        return f"{price:.4f}"
    else:
        # 8 decimals for very small prices
        return f"{price:.8f}"