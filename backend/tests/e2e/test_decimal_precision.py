#!/usr/bin/env python3
"""
Decimal Precision Tests
Validates decimal handling for different token types and ensures data integrity
"""

import struct
import json
import logging
from typing import Dict, List, Any, Tuple
from dataclasses import dataclass
from decimal import Decimal, getcontext

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Set high precision for decimal calculations
getcontext().prec = 28


@dataclass
class TokenConfig:
    """Configuration for token decimal handling"""
    symbol: str
    decimals: int
    chain: str
    typical_price_range: Tuple[float, float]  # Min, max in USD
    is_stablecoin: bool = False


# Known token configurations
TOKEN_CONFIGS = {
    'USDC': TokenConfig('USDC', 6, 'ethereum', (0.99, 1.01), is_stablecoin=True),
    'USDT': TokenConfig('USDT', 6, 'ethereum', (0.99, 1.01), is_stablecoin=True),
    'DAI': TokenConfig('DAI', 18, 'ethereum', (0.99, 1.01), is_stablecoin=True),
    'WETH': TokenConfig('WETH', 18, 'ethereum', (1000, 10000)),
    'WBTC': TokenConfig('WBTC', 8, 'ethereum', (20000, 100000)),
    'WMATIC': TokenConfig('WMATIC', 18, 'polygon', (0.5, 3.0)),
    'LINK': TokenConfig('LINK', 18, 'ethereum', (5, 50)),
    'AAVE': TokenConfig('AAVE', 18, 'ethereum', (50, 500)),
    'UNI': TokenConfig('UNI', 18, 'ethereum', (3, 30)),
    'SUSHI': TokenConfig('SUSHI', 18, 'ethereum', (0.5, 10)),
}


class DecimalPrecisionTester:
    """Tests decimal precision and conversion accuracy"""
    
    def __init__(self):
        self.test_results = []
        self.fixed_point_decimals = 8  # Protocol uses 8 decimal places for fixed-point
        
    def test_fixed_point_conversion(self, value: float, symbol: str = 'GENERIC') -> Dict[str, Any]:
        """Test conversion from float to fixed-point and back using precision-preserving method"""
        # Convert to fixed-point (8 decimals) using Decimal for exact precision
        value_decimal = Decimal(str(value))
        fixed_point = int(value_decimal * Decimal('100000000'))
        
        # Convert back to float
        recovered = fixed_point / 10**self.fixed_point_decimals
        
        # Calculate precision loss
        diff = abs(value - recovered)
        relative_error = diff / value if value != 0 else 0
        
        # Check if within acceptable tolerance
        # For 8 decimal places, we expect precision up to 1e-8
        passed = diff < 1e-8
        
        result = {
            'symbol': symbol,
            'original': value,
            'fixed_point': fixed_point,
            'recovered': recovered,
            'absolute_error': diff,
            'relative_error': relative_error,
            'passed': passed,
            'message': f"Precision {'OK' if passed else 'LOSS'}: {diff:.12f}"
        }
        
        self.test_results.append(result)
        return result
    
    def test_token_decimal_handling(self, token: str, raw_amount: int) -> Dict[str, Any]:
        """Test decimal handling for specific token types"""
        if token not in TOKEN_CONFIGS:
            return {'error': f'Unknown token: {token}'}
        
        config = TOKEN_CONFIGS[token]
        
        # Convert raw amount to human-readable based on token decimals
        human_amount = Decimal(raw_amount) / Decimal(10**config.decimals)
        
        # Convert to protocol fixed-point (8 decimals) using precision-preserving method
        protocol_value = int(human_amount * Decimal('100000000'))
        
        # Convert back to human-readable
        recovered = protocol_value / 10**self.fixed_point_decimals
        
        # Check if value is in typical range
        in_range = config.typical_price_range[0] <= float(human_amount) <= config.typical_price_range[1]
        
        # For stablecoins, check deviation from $1
        stablecoin_check = True
        if config.is_stablecoin:
            deviation = abs(float(human_amount) - 1.0)
            stablecoin_check = deviation < 0.05  # 5% deviation allowed
        
        result = {
            'token': token,
            'token_decimals': config.decimals,
            'raw_amount': raw_amount,
            'human_amount': float(human_amount),
            'protocol_value': protocol_value,
            'recovered': recovered,
            'in_typical_range': in_range,
            'stablecoin_check': stablecoin_check if config.is_stablecoin else None,
            'passed': in_range and stablecoin_check
        }
        
        self.test_results.append(result)
        return result
    
    def test_price_pair_consistency(self, base: str, quote: str, price: float) -> Dict[str, Any]:
        """Test price consistency for trading pairs"""
        result = {
            'pair': f'{base}/{quote}',
            'price': price,
            'checks': []
        }
        
        # Check 1: Price should be positive
        if price <= 0:
            result['checks'].append({
                'test': 'positive_price',
                'passed': False,
                'message': f'Invalid price: {price}'
            })
        
        # Check 2: Stablecoin pairs should be close to 1.0
        if base in TOKEN_CONFIGS and quote in TOKEN_CONFIGS:
            base_config = TOKEN_CONFIGS[base]
            quote_config = TOKEN_CONFIGS[quote]
            
            if base_config.is_stablecoin and quote_config.is_stablecoin:
                deviation = abs(price - 1.0)
                passed = deviation < 0.01  # 1% max deviation for stablecoin pairs
                result['checks'].append({
                    'test': 'stablecoin_pair',
                    'passed': passed,
                    'message': f'Deviation: {deviation:.4f}'
                })
        
        # Check 3: Price within reasonable bounds
        if base == 'WBTC' and quote in ['USDC', 'USDT']:
            passed = 10000 < price < 200000  # BTC reasonable range
            result['checks'].append({
                'test': 'btc_price_range',
                'passed': passed,
                'message': f'BTC price: ${price:,.2f}'
            })
        elif base == 'WETH' and quote in ['USDC', 'USDT']:
            passed = 500 < price < 20000  # ETH reasonable range
            result['checks'].append({
                'test': 'eth_price_range',
                'passed': passed,
                'message': f'ETH price: ${price:,.2f}'
            })
        
        result['passed'] = all(check['passed'] for check in result['checks']) if result['checks'] else True
        self.test_results.append(result)
        return result
    
    def test_volume_precision(self, volume: float, symbol: str) -> Dict[str, Any]:
        """Test volume calculation precision using precision-preserving method"""
        # Volume should maintain precision through conversions using Decimal
        volume_decimal = Decimal(str(volume))
        fixed_volume = int(volume_decimal * Decimal('100000000'))
        recovered = fixed_volume / 10**self.fixed_point_decimals
        
        diff = abs(volume - recovered)
        
        # Volume specific checks
        checks = []
        
        # Check 1: Non-negative volume
        checks.append({
            'test': 'non_negative',
            'passed': volume >= 0,
            'message': f'Volume: {volume}'
        })
        
        # Check 2: Precision maintained
        checks.append({
            'test': 'precision',
            'passed': diff < 1e-8,
            'message': f'Precision loss: {diff:.12f}'
        })
        
        # Check 3: Reasonable volume (not absurdly high)
        max_reasonable = 1e12  # $1 trillion
        checks.append({
            'test': 'reasonable_volume',
            'passed': volume < max_reasonable,
            'message': f'Volume check: {volume < max_reasonable}'
        })
        
        result = {
            'symbol': symbol,
            'volume': volume,
            'fixed_volume': fixed_volume,
            'recovered': recovered,
            'precision_loss': diff,
            'checks': checks,
            'passed': all(check['passed'] for check in checks)
        }
        
        self.test_results.append(result)
        return result
    
    def validate_binary_message(self, binary_data: bytes) -> Dict[str, Any]:
        """Validate decimal handling in binary protocol message"""
        if len(binary_data) < 64:
            return {'error': 'Message too short'}
        
        # Parse price and volume from binary (assuming trade message format)
        price_raw = struct.unpack('<Q', binary_data[32:40])[0]
        volume_raw = struct.unpack('<Q', binary_data[40:48])[0]
        
        # Convert from fixed-point
        price = price_raw / 10**self.fixed_point_decimals
        volume = volume_raw / 10**self.fixed_point_decimals
        
        # Validate conversions
        price_test = self.test_fixed_point_conversion(price, 'from_binary')
        volume_test = self.test_volume_precision(volume, 'from_binary')
        
        return {
            'price_raw': price_raw,
            'volume_raw': volume_raw,
            'price': price,
            'volume': volume,
            'price_validation': price_test,
            'volume_validation': volume_test,
            'passed': price_test['passed'] and volume_test['passed']
        }
    
    def run_comprehensive_tests(self) -> Dict[str, Any]:
        """Run comprehensive decimal precision tests"""
        results = {
            'fixed_point_tests': [],
            'token_tests': [],
            'pair_tests': [],
            'edge_cases': []
        }
        
        # Test 1: Fixed-point conversion for various values
        test_values = [
            0.00000001,  # Minimum precision
            0.12345678,  # 8 decimal places
            1.0,
            1234.56789,
            99999999.99999999,  # Large value
            0.99999999,  # Close to 1
            3.14159265,  # Pi
        ]
        
        for val in test_values:
            result = self.test_fixed_point_conversion(val)
            results['fixed_point_tests'].append(result)
        
        # Test 2: Token-specific decimal handling (using realistic USD prices)
        token_tests = [
            ('USDC', 1000000),     # 1 USDC (6 decimals) = $1.00
            ('USDT', 1000000),     # 1 USDT (6 decimals) = $1.00
            ('WETH', int(4605.23 * 10**18)),  # ~$4605 worth of ETH (18 decimals)
            ('WBTC', int(68234.56 * 10**8)),  # ~$68234 worth of BTC (8 decimals)  
            ('DAI', 1000000000000000000),   # 1 DAI (18 decimals) = $1.00
        ]
        
        for token, raw_amount in token_tests:
            result = self.test_token_decimal_handling(token, raw_amount)
            results['token_tests'].append(result)
        
        # Test 3: Price pair consistency
        pair_tests = [
            ('USDC', 'USDT', 0.9999),
            ('WETH', 'USDC', 4605.23),
            ('WBTC', 'USDC', 68234.56),
            ('WMATIC', 'USDC', 1.23),
        ]
        
        for base, quote, price in pair_tests:
            result = self.test_price_pair_consistency(base, quote, price)
            results['pair_tests'].append(result)
        
        # Test 4: Edge cases
        edge_cases = [
            {'test': 'zero', 'value': 0.0},
            {'test': 'very_small', 'value': 1e-10},
            {'test': 'very_large', 'value': 1e15},
            {'test': 'max_uint64', 'value': 2**64 - 1},
        ]
        
        for case in edge_cases:
            try:
                result = self.test_fixed_point_conversion(case['value'], case['test'])
                results['edge_cases'].append({
                    'case': case['test'],
                    'result': result,
                    'error': None
                })
            except Exception as e:
                results['edge_cases'].append({
                    'case': case['test'],
                    'result': None,
                    'error': str(e)
                })
        
        # Summary
        all_tests = self.test_results
        passed_count = sum(1 for t in all_tests if t.get('passed', False))
        
        results['summary'] = {
            'total_tests': len(all_tests),
            'passed': passed_count,
            'failed': len(all_tests) - passed_count,
            'pass_rate': passed_count / len(all_tests) if all_tests else 1.0
        }
        
        return results
    
    def generate_report(self) -> str:
        """Generate human-readable test report"""
        results = self.run_comprehensive_tests()
        
        report = []
        report.append("=" * 60)
        report.append("DECIMAL PRECISION TEST REPORT")
        report.append("=" * 60)
        
        summary = results['summary']
        report.append(f"\nSummary:")
        report.append(f"  Total Tests: {summary['total_tests']}")
        report.append(f"  Passed: {summary['passed']}")
        report.append(f"  Failed: {summary['failed']}")
        report.append(f"  Pass Rate: {summary['pass_rate']:.1%}")
        
        # Fixed-point tests
        report.append(f"\nFixed-Point Conversion Tests:")
        for test in results['fixed_point_tests'][:5]:  # Show first 5
            status = "✅" if test['passed'] else "❌"
            report.append(f"  {status} {test['original']} → {test['recovered']} (error: {test['absolute_error']:.12f})")
        
        # Token tests
        report.append(f"\nToken Decimal Handling:")
        for test in results['token_tests']:
            status = "✅" if test['passed'] else "❌"
            report.append(f"  {status} {test['token']}: {test['human_amount']:.8f} (decimals: {test['token_decimals']})")
        
        # Pair tests
        report.append(f"\nPrice Pair Validation:")
        for test in results['pair_tests']:
            status = "✅" if test['passed'] else "❌"
            report.append(f"  {status} {test['pair']}: ${test['price']:,.2f}")
            for check in test['checks']:
                check_status = "✓" if check['passed'] else "✗"
                report.append(f"    {check_status} {check['test']}: {check['message']}")
        
        return "\n".join(report)


def main():
    """Run decimal precision tests"""
    tester = DecimalPrecisionTester()
    
    # Run comprehensive tests
    results = tester.run_comprehensive_tests()
    
    # Print report
    print(tester.generate_report())
    
    # Save results
    with open('decimal_precision_report.json', 'w') as f:
        json.dump(results, f, indent=2, default=str)
    
    logger.info("Decimal precision tests completed")
    
    # Return exit code based on results
    return 0 if results['summary']['pass_rate'] >= 0.95 else 1


if __name__ == "__main__":
    import sys
    sys.exit(main())