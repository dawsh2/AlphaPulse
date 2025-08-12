# Basic Cross-Exchange Arbitrage Analysis Template
# This template provides starter code for analyzing arbitrage opportunities

template = {
    "title": "Cross-Exchange Arbitrage Analysis",
    "description": "Analyze price differences between Coinbase and Kraken",
    "cells": [
        {
            "type": "markdown",
            "content": "# Cross-Exchange Arbitrage Analysis\n\nThis notebook analyzes BTC/USD price differences between exchanges to identify arbitrage opportunities."
        },
        {
            "type": "code",
            "content": """# Import required libraries
import pandas as pd
import numpy as np
import duckdb
import matplotlib.pyplot as plt
from datetime import datetime, timedelta

print("Libraries loaded successfully!")"""
        },
        {
            "type": "code", 
            "content": """# Connect to market data (read-only mode to avoid lock conflicts)
conn = duckdb.connect('../backend/market_data/market_data.duckdb', read_only=True)

# Load BTC data from both exchanges
query = '''
    SELECT 
        timestamp,
        symbol,
        exchange,
        close as price,
        volume
    FROM ohlcv
    WHERE symbol = 'BTC/USD'
    ORDER BY timestamp DESC
    LIMIT 10000
'''

df = conn.execute(query).df()
# Convert timestamp from seconds to datetime
df['timestamp'] = pd.to_datetime(df['timestamp'], unit='s')
print(f"Loaded {len(df)} records")
print(f"Exchanges: {df['exchange'].unique()}")
print(f"Date range: {df['timestamp'].min()} to {df['timestamp'].max()}")"""
        },
        {
            "type": "code",
            "content": """# Pivot data for comparison
price_comparison = df.pivot_table(
    index='timestamp',
    columns='exchange',
    values='price',
    aggfunc='mean'
).dropna()

print(f"\\nComparable data points: {len(price_comparison)}")
price_comparison.tail()"""
        },
        {
            "type": "code",
            "content": """# Calculate spreads
price_comparison['spread_usd'] = price_comparison['coinbase'] - price_comparison['kraken']
price_comparison['spread_pct'] = (price_comparison['spread_usd'] / price_comparison['kraken']) * 100

# Summary statistics
print("=== Spread Statistics ===")
print(f"Mean spread: ${price_comparison['spread_usd'].mean():.2f}")
print(f"Max spread: ${price_comparison['spread_usd'].max():.2f}")
print(f"Min spread: ${price_comparison['spread_usd'].min():.2f}")
print(f"Std deviation: ${price_comparison['spread_usd'].std():.2f}")
print(f"\\nMean spread %: {price_comparison['spread_pct'].mean():.3f}%")
print(f"Max spread %: {price_comparison['spread_pct'].max():.3f}%")"""
        },
        {
            "type": "code",
            "content": """# Identify arbitrage opportunities (> 0.1% spread after fees)
# Assuming 0.05% fee per exchange = 0.1% total
min_profit_threshold = 0.1  # percentage

opportunities = price_comparison[abs(price_comparison['spread_pct']) > min_profit_threshold].copy()
opportunities['direction'] = opportunities['spread_pct'].apply(
    lambda x: 'Buy Kraken, Sell Coinbase' if x > 0 else 'Buy Coinbase, Sell Kraken'
)

print(f"\\n=== Arbitrage Opportunities ===")
print(f"Found {len(opportunities)} opportunities (>{min_profit_threshold}% spread)")
print(f"Percentage of time: {len(opportunities)/len(price_comparison)*100:.2f}%")

if len(opportunities) > 0:
    print(f"\\nTop 5 opportunities:")
    top_opps = opportunities.nlargest(5, 'spread_pct')[['coinbase', 'kraken', 'spread_usd', 'spread_pct', 'direction']]
    print(top_opps)"""
        },
        {
            "type": "code",
            "content": """# Visualize spreads over time
import matplotlib.pyplot as plt
%matplotlib inline

fig, axes = plt.subplots(3, 1, figsize=(12, 10))

# Price comparison
axes[0].plot(price_comparison.index, price_comparison['coinbase'], label='Coinbase', alpha=0.7)
axes[0].plot(price_comparison.index, price_comparison['kraken'], label='Kraken', alpha=0.7)
axes[0].set_ylabel('Price (USD)')
axes[0].set_title('BTC/USD Prices by Exchange')
axes[0].legend()
axes[0].grid(True, alpha=0.3)

# Absolute spread
axes[1].plot(price_comparison.index, price_comparison['spread_usd'], color='purple', alpha=0.7)
axes[1].axhline(y=0, color='black', linestyle='-', alpha=0.3)
axes[1].fill_between(price_comparison.index, price_comparison['spread_usd'], 0, alpha=0.3)
axes[1].set_ylabel('Spread (USD)')
axes[1].set_title('Price Spread (Coinbase - Kraken)')
axes[1].grid(True, alpha=0.3)

# Percentage spread with threshold lines
axes[2].plot(price_comparison.index, price_comparison['spread_pct'], color='green', alpha=0.7)
axes[2].axhline(y=min_profit_threshold, color='red', linestyle='--', alpha=0.5, label=f'Profit threshold ({min_profit_threshold}%)')
axes[2].axhline(y=-min_profit_threshold, color='red', linestyle='--', alpha=0.5)
axes[2].axhline(y=0, color='black', linestyle='-', alpha=0.3)
axes[2].fill_between(price_comparison.index, price_comparison['spread_pct'], 0, alpha=0.3)
axes[2].set_ylabel('Spread (%)')
axes[2].set_xlabel('Time')
axes[2].set_title('Percentage Spread with Arbitrage Threshold')
axes[2].legend()
axes[2].grid(True, alpha=0.3)

plt.tight_layout()
plt.show()

print("Charts generated successfully!")"""
        },
        {
            "type": "code",
            "content": """# Calculate potential profits (simplified)
# Assuming $10,000 capital per trade

capital = 10000
fee_rate = 0.0005  # 0.05% per exchange
total_fees = fee_rate * 2  # Buy and sell

# Calculate profit for each opportunity
opportunities['gross_profit'] = capital * (abs(opportunities['spread_pct']) / 100)
opportunities['fees'] = capital * total_fees
opportunities['net_profit'] = opportunities['gross_profit'] - opportunities['fees']

total_profit = opportunities['net_profit'].sum()
avg_profit_per_opp = opportunities['net_profit'].mean()

print(f"\\n=== Profit Analysis (${capital} per trade) ===")
print(f"Total potential profit: ${total_profit:.2f}")
print(f"Average profit per opportunity: ${avg_profit_per_opp:.2f}")
print(f"Number of profitable trades: {len(opportunities[opportunities['net_profit'] > 0])}")
print(f"Success rate: {len(opportunities[opportunities['net_profit'] > 0])/len(opportunities)*100:.1f}%")"""
        },
        {
            "type": "markdown",
            "content": """## Next Steps

1. **Add more sophisticated analysis:**
   - Include order book depth
   - Account for slippage
   - Consider transfer times between exchanges
   
2. **Expand to more pairs:**
   - ETH/USD on both exchanges
   - Other cryptocurrency pairs
   
3. **Build execution strategy:**
   - Define entry/exit rules
   - Risk management parameters
   - Position sizing logic"""
        }
    ]
}