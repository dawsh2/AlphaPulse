#!/usr/bin/env python3
"""
Data Comparison Engine
Compares captured data from WebSocket bridge output to validate data integrity
"""

import json
import time
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass
import logging
from collections import defaultdict
import statistics

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@dataclass
class ComparisonResult:
    """Result of comparing two data points"""
    symbol: str
    field: str
    expected: Any
    actual: Any
    difference: float
    relative_diff: float
    passed: bool
    message: str


class DataComparisonEngine:
    """Compares WebSocket data across multiple capture points"""
    
    def __init__(self, tolerance: float = 0.0001):
        """
        Initialize comparison engine
        
        Args:
            tolerance: Maximum relative difference allowed (0.0001 = 0.01%)
        """
        self.tolerance = tolerance
        self.results: List[ComparisonResult] = []
        self.ws_messages: List[Dict] = []
        self.binary_messages: List[Dict] = []
        self.symbol_hash_map: Dict[int, str] = {}  # Maps hash to symbol string
        
    def load_ws_capture(self, filepath: str):
        """Load WebSocket capture from file"""
        with open(filepath, 'r') as f:
            data = json.load(f)
        self.ws_messages = data.get('messages', [])
        logger.info(f"Loaded {len(self.ws_messages)} WebSocket messages")
        return data.get('capture_info', {}), data.get('statistics', {})
    
    def load_binary_capture(self, filepath: str):
        """Load binary protocol capture from file"""
        with open(filepath, 'r') as f:
            data = json.load(f)
        self.binary_messages = data.get('messages', [])
        logger.info(f"Loaded {len(self.binary_messages)} binary messages")
        return data.get('capture_info', {}), data.get('statistics', {})
    
    def compare_price_consistency(self) -> List[ComparisonResult]:
        """Check if prices for the same symbol are consistent within tolerance"""
        results = []
        
        # Group WebSocket messages by symbol and time window
        symbol_prices = defaultdict(list)
        
        for msg in self.ws_messages:
            if msg.get('msg_type') == 'trade' and msg.get('price'):
                symbol = msg.get('symbol')
                if symbol:
                    symbol_prices[symbol].append({
                        'price': msg['price'],
                        'timestamp': msg['timestamp'],
                        'volume': msg.get('volume'),
                        'exchange': msg.get('exchange')
                    })
        
        # Check consistency within each symbol
        for symbol, prices in symbol_prices.items():
            if len(prices) < 2:
                continue
                
            # Sort by timestamp
            prices.sort(key=lambda x: x['timestamp'])
            
            # Check sequential price changes
            for i in range(1, len(prices)):
                prev = prices[i-1]
                curr = prices[i]
                
                # Calculate price change
                price_diff = abs(curr['price'] - prev['price'])
                time_diff = curr['timestamp'] - prev['timestamp']
                
                # Check if price change is reasonable (not a data error)
                if prev['price'] > 0:
                    relative_change = price_diff / prev['price']
                    
                    # Flag large price jumps in short time (potential data error)
                    if time_diff < 1000 and relative_change > 0.1:  # >10% change in <1 second
                        results.append(ComparisonResult(
                            symbol=symbol,
                            field='price_continuity',
                            expected=prev['price'],
                            actual=curr['price'],
                            difference=price_diff,
                            relative_diff=relative_change,
                            passed=False,
                            message=f"Large price jump: {relative_change:.2%} in {time_diff}ms"
                        ))
        
        return results
    
    def compare_decimal_handling(self) -> List[ComparisonResult]:
        """Validate decimal handling for different token types"""
        results = []
        
        # Known decimal places for tokens
        token_decimals = {
            'USDC': 6,
            'USDT': 6,
            'WBTC': 8,
            'WETH': 18,
            'DAI': 18,
            'WMATIC': 18,
            'LINK': 18,
            'AAVE': 18
        }
        
        for msg in self.ws_messages:
            if msg.get('msg_type') == 'trade' and msg.get('price'):
                symbol = msg.get('symbol', '')
                price = msg.get('price')
                
                # Extract token from symbol (e.g., "quickswap:WETH-USDC")
                if ':' in symbol and '-' in symbol:
                    tokens = symbol.split(':')[1].split('-')
                    
                    # Check if price is reasonable for the token pair
                    for token in tokens:
                        if token in token_decimals:
                            # Basic sanity checks
                            if token in ['WETH', 'WBTC'] and price > 1000000:
                                results.append(ComparisonResult(
                                    symbol=symbol,
                                    field='decimal_handling',
                                    expected="< 1000000",
                                    actual=price,
                                    difference=price - 1000000,
                                    relative_diff=1.0,
                                    passed=False,
                                    message=f"Unrealistic price for {token}: ${price:,.2f}"
                                ))
                            elif token in ['USDC', 'USDT', 'DAI'] and abs(price - 1.0) > 0.1:
                                # Stablecoins should be close to $1
                                if 'USDC' in tokens and 'USDT' in tokens:
                                    # USDC-USDT pair should be very close to 1.0
                                    if abs(price - 1.0) > 0.01:
                                        results.append(ComparisonResult(
                                            symbol=symbol,
                                            field='stablecoin_price',
                                            expected=1.0,
                                            actual=price,
                                            difference=abs(price - 1.0),
                                            relative_diff=abs(price - 1.0),
                                            passed=False,
                                            message=f"Stablecoin pair price deviation: ${price:.4f}"
                                        ))
        
        return results
    
    def compare_binary_to_json(self) -> List[ComparisonResult]:
        """Compare binary protocol messages to JSON WebSocket output"""
        results = []
        
        # Build symbol mapping from WebSocket messages
        for msg in self.ws_messages:
            if msg.get('msg_type') == 'symbol_mapping':
                hash_val = msg.get('symbol_hash')
                symbol = msg.get('symbol')
                if hash_val and symbol:
                    try:
                        self.symbol_hash_map[int(hash_val)] = symbol
                    except ValueError:
                        pass
        
        # Group messages by type and timestamp for matching
        binary_trades = {}
        json_trades = {}
        
        # Process binary messages
        for msg in self.binary_messages:
            if msg.get('msg_type') == 1:  # Trade
                symbol_hash = msg.get('symbol_hash')
                if symbol_hash:
                    key = (symbol_hash, int(msg['timestamp']))
                    if key not in binary_trades:
                        binary_trades[key] = []
                    binary_trades[key].append(msg)
        
        # Process JSON messages
        for msg in self.ws_messages:
            if msg.get('msg_type') == 'trade':
                symbol_hash = msg.get('symbol_hash')
                if symbol_hash:
                    try:
                        hash_int = int(symbol_hash)
                        timestamp = msg.get('timestamp', 0) / 1000  # Convert ms to seconds
                        key = (hash_int, int(timestamp))
                        if key not in json_trades:
                            json_trades[key] = []
                        json_trades[key].append(msg)
                    except (ValueError, TypeError):
                        pass
        
        # Compare matching messages
        for key in set(binary_trades.keys()) & set(json_trades.keys()):
            symbol_hash, _ = key
            symbol = self.symbol_hash_map.get(symbol_hash, f"hash_{symbol_hash}")
            
            for binary_msg in binary_trades[key]:
                for json_msg in json_trades[key]:
                    # Compare price
                    binary_price = binary_msg.get('price_float', 0)
                    json_price = json_msg.get('price', 0)
                    
                    if binary_price > 0:
                        price_diff = abs(binary_price - json_price)
                        relative_diff = price_diff / binary_price
                        
                        results.append(ComparisonResult(
                            symbol=symbol,
                            field='price_binary_to_json',
                            expected=binary_price,
                            actual=json_price,
                            difference=price_diff,
                            relative_diff=relative_diff,
                            passed=relative_diff <= self.tolerance,
                            message=f"Binary: {binary_price:.8f}, JSON: {json_price:.8f}"
                        ))
                    
                    # Compare volume
                    binary_volume = binary_msg.get('volume_float', 0)
                    json_volume = json_msg.get('volume', 0)
                    
                    if binary_volume > 0:
                        volume_diff = abs(binary_volume - json_volume)
                        relative_diff = volume_diff / binary_volume
                        
                        results.append(ComparisonResult(
                            symbol=symbol,
                            field='volume_binary_to_json',
                            expected=binary_volume,
                            actual=json_volume,
                            difference=volume_diff,
                            relative_diff=relative_diff,
                            passed=relative_diff <= self.tolerance,
                            message=f"Binary: {binary_volume:.8f}, JSON: {json_volume:.8f}"
                        ))
        
        return results
    
    def compare_fixed_point_conversion(self) -> List[ComparisonResult]:
        """Validate fixed-point to floating-point conversion"""
        results = []
        
        for msg in self.binary_messages:
            if msg.get('msg_type') == 1:  # Trade
                price_raw = msg.get('price_raw')
                price_float = msg.get('price_float')
                
                if price_raw is not None and price_float is not None:
                    # Calculate expected float from raw
                    expected_float = price_raw / 100000000  # 8 decimal places
                    
                    diff = abs(expected_float - price_float)
                    
                    results.append(ComparisonResult(
                        symbol=self.symbol_hash_map.get(msg.get('symbol_hash'), 'unknown'),
                        field='fixed_point_conversion',
                        expected=expected_float,
                        actual=price_float,
                        difference=diff,
                        relative_diff=diff / expected_float if expected_float > 0 else 0,
                        passed=diff < 1e-10,  # Very tight tolerance for conversion
                        message=f"Raw: {price_raw}, Expected: {expected_float:.8f}, Actual: {price_float:.8f}"
                    ))
        
        return results
    
    def compare_arbitrage_calculations(self) -> List[ComparisonResult]:
        """Validate arbitrage opportunity calculations"""
        results = []
        
        # Group prices by base token to find arbitrage
        base_token_prices = defaultdict(list)
        
        for msg in self.ws_messages:
            if msg.get('msg_type') == 'trade' and msg.get('price'):
                symbol = msg.get('symbol', '')
                if ':' in symbol and '-' in symbol:
                    exchange, pair = symbol.split(':')
                    tokens = pair.split('-')
                    if len(tokens) == 2:
                        base_token = tokens[0]
                        quote_token = tokens[1]
                        
                        base_token_prices[base_token].append({
                            'exchange': exchange,
                            'pair': pair,
                            'quote': quote_token,
                            'price': msg['price'],
                            'timestamp': msg['timestamp'],
                            'symbol': symbol
                        })
        
        # Check for arbitrage opportunities
        for base_token, prices in base_token_prices.items():
            # Group by quote token
            by_quote = defaultdict(list)
            for p in prices:
                by_quote[p['quote']].append(p)
            
            # Check price differences within same quote
            for quote_token, quote_prices in by_quote.items():
                if len(quote_prices) < 2:
                    continue
                
                # Find min and max prices
                min_price = min(quote_prices, key=lambda x: x['price'])
                max_price = max(quote_prices, key=lambda x: x['price'])
                
                if min_price['price'] > 0:
                    spread = (max_price['price'] - min_price['price']) / min_price['price']
                    
                    # Estimate gas cost (hardcoded as in frontend)
                    gas_cost = 0.10  # $0.10 for 2 swaps on Polygon
                    gross_profit = max_price['price'] - min_price['price']
                    net_profit = gross_profit - gas_cost
                    
                    # Log significant arbitrage opportunities
                    if spread > 0.001:  # >0.1% spread
                        results.append(ComparisonResult(
                            symbol=f"{base_token}-{quote_token}",
                            field='arbitrage_opportunity',
                            expected=min_price['price'],
                            actual=max_price['price'],
                            difference=gross_profit,
                            relative_diff=spread,
                            passed=True,  # This is informational
                            message=f"Arbitrage: {min_price['exchange']} (${min_price['price']:.2f}) → {max_price['exchange']} (${max_price['price']:.2f}), Net profit: ${net_profit:.2f}"
                        ))
        
        return results
    
    def compare_latency_measurements(self) -> Dict[str, Any]:
        """Analyze latency measurements from WebSocket messages"""
        latencies = {
            'collector_to_relay': [],
            'relay_to_bridge': [],
            'bridge_to_frontend': [],
            'total': []
        }
        
        for msg in self.ws_messages:
            if msg.get('msg_type') == 'trade':
                if msg.get('latency_collector_to_relay_us'):
                    latencies['collector_to_relay'].append(msg['latency_collector_to_relay_us'] / 1000)  # Convert to ms
                if msg.get('latency_relay_to_bridge_us'):
                    latencies['relay_to_bridge'].append(msg['latency_relay_to_bridge_us'] / 1000)
                if msg.get('latency_bridge_to_frontend_us'):
                    latencies['bridge_to_frontend'].append(msg['latency_bridge_to_frontend_us'] / 1000)
                if msg.get('latency_total_us'):
                    latencies['total'].append(msg['latency_total_us'] / 1000)
        
        stats = {}
        for stage, values in latencies.items():
            if values:
                stats[stage] = {
                    'min_ms': min(values),
                    'max_ms': max(values),
                    'avg_ms': statistics.mean(values),
                    'median_ms': statistics.median(values),
                    'p95_ms': sorted(values)[int(len(values) * 0.95)] if len(values) > 20 else max(values),
                    'count': len(values)
                }
        
        return stats
    
    def run_all_comparisons(self) -> Dict[str, Any]:
        """Run all comparison tests"""
        price_consistency = self.compare_price_consistency()
        decimal_handling = self.compare_decimal_handling()
        arbitrage = self.compare_arbitrage_calculations()
        latency = self.compare_latency_measurements()
        
        # New comparison methods
        binary_to_json = self.compare_binary_to_json() if self.binary_messages else []
        fixed_point = self.compare_fixed_point_conversion() if self.binary_messages else []
        
        all_results = price_consistency + decimal_handling + arbitrage + binary_to_json + fixed_point
        
        passed = sum(1 for r in all_results if r.passed)
        failed = sum(1 for r in all_results if not r.passed)
        
        return {
            'summary': {
                'total_tests': len(all_results),
                'passed': passed,
                'failed': failed,
                'pass_rate': passed / len(all_results) if all_results else 1.0,
                'ws_messages_analyzed': len(self.ws_messages),
                'binary_messages_analyzed': len(self.binary_messages)
            },
            'price_consistency': {
                'issues': [r for r in price_consistency if not r.passed],
                'count': len(price_consistency)
            },
            'decimal_handling': {
                'issues': [r for r in decimal_handling if not r.passed],
                'count': len(decimal_handling)
            },
            'binary_to_json_validation': {
                'issues': [r for r in binary_to_json if not r.passed],
                'count': len(binary_to_json),
                'pass_rate': sum(1 for r in binary_to_json if r.passed) / len(binary_to_json) if binary_to_json else 1.0
            },
            'fixed_point_conversion': {
                'issues': [r for r in fixed_point if not r.passed],
                'count': len(fixed_point),
                'pass_rate': sum(1 for r in fixed_point if r.passed) / len(fixed_point) if fixed_point else 1.0
            },
            'arbitrage_opportunities': [
                {
                    'symbol': r.symbol,
                    'spread': f"{r.relative_diff:.2%}",
                    'profit': f"${r.difference:.2f}",
                    'details': r.message
                }
                for r in arbitrage if r.relative_diff > 0.001
            ],
            'latency_stats': latency,
            'failed_tests': [
                {
                    'symbol': r.symbol,
                    'field': r.field,
                    'expected': r.expected,
                    'actual': r.actual,
                    'message': r.message
                }
                for r in all_results if not r.passed
            ][:10]  # First 10 failures
        }
    
    def save_report(self, filepath: str):
        """Save comparison report to file"""
        report = self.run_all_comparisons()
        
        with open(filepath, 'w') as f:
            json.dump(report, f, indent=2, default=str)
        
        logger.info(f"Saved comparison report to {filepath}")
        return report


def main():
    """Example usage"""
    engine = DataComparisonEngine(tolerance=0.0001)
    
    # Load captured data
    # engine.load_ws_capture("captured_ws_data.json")
    # engine.load_binary_capture("binary_capture.json")
    
    # For testing with sample data
    engine.ws_messages = [
        {
            'msg_type': 'trade',
            'symbol': 'quickswap:WETH-USDC',
            'price': 4605.23,
            'volume': 1.5,
            'timestamp': time.time() * 1000,
            'latency_total_us': 1500
        },
        {
            'msg_type': 'trade', 
            'symbol': 'sushiswap:WETH-USDC',
            'price': 4608.91,
            'volume': 2.1,
            'timestamp': time.time() * 1000,
            'latency_total_us': 1200
        },
        {
            'msg_type': 'trade',
            'symbol': 'quickswap:USDC-USDT',
            'price': 0.9998,
            'volume': 10000,
            'timestamp': time.time() * 1000,
            'latency_total_us': 800
        }
    ]
    
    # Run comparisons
    report = engine.run_all_comparisons()
    print(json.dumps(report, indent=2))
    
    # Save report
    engine.save_report("comparison_report.json")


if __name__ == "__main__":
    main()