#!/usr/bin/env python3
"""
Analyze why EMA Cross strategy is generating so few signals.
"""

from pathlib import Path
import pandas as pd
import numpy as np
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
    
    # Create EMAs manually
    fast_ema = ExponentialMovingAverage(10)
    slow_ema = ExponentialMovingAverage(20)
    
    # Track signals
    signals = []
    prices = []
    fast_values = []
    slow_values = []
    
    # Skip single-price bars count
    skipped_bars = 0
    
    for i, bar in enumerate(bars):
        # Skip single-price bars like the strategy does
        if bar.is_single_price():
            skipped_bars += 1
            continue
            
        # Update EMAs
        fast_ema.update_raw(float(bar.close))
        slow_ema.update_raw(float(bar.close))
        
        prices.append(float(bar.close))
        
        # Only track after warmup
        if fast_ema.initialized and slow_ema.initialized:
            fast_val = fast_ema.value
            slow_val = slow_ema.value
            fast_values.append(fast_val)
            slow_values.append(slow_val)
            
            # Check for crossovers
            if i > 0 and len(fast_values) > 1:
                prev_fast = fast_values[-2]
                prev_slow = slow_values[-2]
                
                # Bullish crossover (fast crosses above slow)
                if prev_fast <= prev_slow and fast_val > slow_val:
                    signals.append({
                        'bar_index': i,
                        'type': 'BUY',
                        'price': float(bar.close),
                        'fast_ema': fast_val,
                        'slow_ema': slow_val,
                        'timestamp': bar.ts_event
                    })
                
                # Bearish crossover (fast crosses below slow)
                elif prev_fast >= prev_slow and fast_val < slow_val:
                    signals.append({
                        'bar_index': i,
                        'type': 'SELL',
                        'price': float(bar.close),
                        'fast_ema': fast_val,
                        'slow_ema': slow_val,
                        'timestamp': bar.ts_event
                    })
    
    print(f"\nSkipped {skipped_bars} single-price bars")
    print(f"Processed {len(bars) - skipped_bars} bars with price movement")
    
    # Show EMA convergence
    if fast_values and slow_values:
        print(f"\nEMA Analysis:")
        print(f"Initial Fast EMA: {fast_values[0]:.2f}")
        print(f"Initial Slow EMA: {slow_values[0]:.2f}")
        print(f"Initial difference: {abs(fast_values[0] - slow_values[0]):.2f}")
        
        print(f"\nFinal Fast EMA: {fast_values[-1]:.2f}")
        print(f"Final Slow EMA: {slow_values[-1]:.2f}")
        print(f"Final difference: {abs(fast_values[-1] - slow_values[-1]):.2f}")
        
        # Calculate how often they're close
        differences = [abs(f - s) for f, s in zip(fast_values, slow_values)]
        avg_diff = np.mean(differences)
        min_diff = min(differences)
        max_diff = max(differences)
        
        print(f"\nEMA Spread Statistics:")
        print(f"Average difference: ${avg_diff:.2f}")
        print(f"Min difference: ${min_diff:.2f}")
        print(f"Max difference: ${max_diff:.2f}")
        
        # Check price range
        price_range = max(prices) - min(prices)
        price_volatility = np.std(prices)
        print(f"\nPrice Statistics:")
        print(f"Price range: ${min(prices):.2f} - ${max(prices):.2f} (${price_range:.2f})")
        print(f"Price volatility (std): ${price_volatility:.2f}")
    
    # Show signals
    print(f"\nTotal Signals Generated: {len(signals)}")
    
    if signals:
        print("\nSignal Details:")
        for i, signal in enumerate(signals):
            from datetime import datetime
            ts = datetime.fromtimestamp(signal['timestamp'] / 1e9)
            print(f"{i+1}. {signal['type']} at {ts:%Y-%m-%d %H:%M} - Price: ${signal['price']:.2f}")
            print(f"   Fast EMA: ${signal['fast_ema']:.2f}, Slow EMA: ${signal['slow_ema']:.2f}")
    
    # Analyze why there might be few signals
    print("\nPossible reasons for few signals:")
    print("1. Short time period (5 days) - not enough time for multiple crossovers")
    print("2. Strong trend - EMAs don't cross often in trending markets")
    print("3. EMA periods (10/20) might be too close together")
    
    # Show a sample of data to verify
    print("\nSample of price movements (every 100 bars):")
    for i in range(0, len(prices), 100):
        if i < len(prices):
            print(f"  Bar {i}: ${prices[i]:.2f}")


if __name__ == "__main__":
    main()