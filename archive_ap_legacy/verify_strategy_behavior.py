#!/usr/bin/env python3
"""
Verify the actual EMA Cross strategy behavior vs crossover detection.
"""

from pathlib import Path
import pandas as pd
from nautilus_trader.persistence.catalog import ParquetDataCatalog
from nautilus_trader.model.data import Bar
from nautilus_trader.indicators.average.ema import ExponentialMovingAverage


def main():
    # Load data
    catalog_path = Path.cwd() / "catalog"
    catalog = ParquetDataCatalog(catalog_path)
    
    # Query data
    bar_type_str = "NVDA.ALPACA-1-MINUTE-LAST-EXTERNAL"
    bars = catalog.query(
        data_cls=Bar,
        identifiers=[bar_type_str],
    )
    
    print(f"Total bars: {len(bars)}")
    
    # Create EMAs
    fast_ema = ExponentialMovingAverage(10)
    slow_ema = ExponentialMovingAverage(20)
    
    # Track strategy state
    position = "FLAT"  # FLAT, LONG, SHORT
    trades = []
    
    for i, bar in enumerate(bars):
        # Skip single-price bars
        if bar.is_single_price():
            continue
            
        # Update EMAs
        fast_ema.update_raw(float(bar.close))
        slow_ema.update_raw(float(bar.close))
        
        # Only trade after warmup
        if fast_ema.initialized and slow_ema.initialized:
            fast_val = fast_ema.value
            slow_val = slow_ema.value
            
            # Implement the ACTUAL strategy logic from ema_cross.py
            if fast_val >= slow_val:
                # Strategy wants to be LONG
                if position == "FLAT":
                    # BUY
                    trades.append({
                        'bar': i,
                        'action': 'BUY',
                        'price': float(bar.close),
                        'reason': 'Open long from flat'
                    })
                    position = "LONG"
                elif position == "SHORT":
                    # Close short and go long
                    trades.append({
                        'bar': i,
                        'action': 'CLOSE_SHORT',
                        'price': float(bar.close),
                        'reason': 'Close short position'
                    })
                    trades.append({
                        'bar': i,
                        'action': 'BUY',
                        'price': float(bar.close),
                        'reason': 'Open long after closing short'
                    })
                    position = "LONG"
            else:  # fast_val < slow_val
                # Strategy wants to be SHORT
                if position == "FLAT":
                    # SELL
                    trades.append({
                        'bar': i,
                        'action': 'SELL',
                        'price': float(bar.close),
                        'reason': 'Open short from flat'
                    })
                    position = "SHORT"
                elif position == "LONG":
                    # Close long and go short
                    trades.append({
                        'bar': i,
                        'action': 'CLOSE_LONG',
                        'price': float(bar.close),
                        'reason': 'Close long position'
                    })
                    trades.append({
                        'bar': i,
                        'action': 'SELL',
                        'price': float(bar.close),
                        'reason': 'Open short after closing long'
                    })
                    position = "SHORT"
    
    print(f"\nStrategy Simulation Results:")
    print(f"Total trades: {len(trades)}")
    print(f"Final position: {position}")
    
    # Show first 20 trades
    print(f"\nFirst 20 trades:")
    for i, trade in enumerate(trades[:20]):
        print(f"{i+1}. Bar {trade['bar']}: {trade['action']} @ ${trade['price']:.2f} - {trade['reason']}")
    
    # Count trade types
    buys = sum(1 for t in trades if t['action'] == 'BUY')
    sells = sum(1 for t in trades if t['action'] == 'SELL')
    closes = sum(1 for t in trades if 'CLOSE' in t['action'])
    
    print(f"\nTrade Summary:")
    print(f"BUY orders: {buys}")
    print(f"SELL orders: {sells}")
    print(f"CLOSE orders: {closes}")
    print(f"Total orders: {len(trades)}")
    
    # Calculate round trips
    round_trips = min(buys, sells)
    print(f"\nEstimated round trips (positions opened and closed): {round_trips}")


if __name__ == "__main__":
    main()