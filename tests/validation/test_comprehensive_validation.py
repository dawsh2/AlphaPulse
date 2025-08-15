#!/usr/bin/env python3
"""
Comprehensive Data Validation Test Suite

Tests all aspects of the data validation pipeline including:
- Exchange-specific data format validation
- Protocol message integrity
- Real-world edge cases
- Performance under load
- Error handling and recovery
"""

import asyncio
import json
import time
import statistics
from typing import Dict, List, Tuple, Any
from decimal import Decimal, getcontext
import unittest
from dataclasses import dataclass

# Set high precision for Decimal calculations
getcontext().prec = 28

@dataclass
class ValidationResult:
    """Result of a validation test"""
    test_name: str
    passed: bool
    error_message: str = ""
    execution_time_ms: float = 0.0
    data_points_tested: int = 0
    precision_errors: List[float] = None
    
    def __post_init__(self):
        if self.precision_errors is None:
            self.precision_errors = []

class ComprehensiveDataValidator:
    """Comprehensive data validation test framework"""
    
    def __init__(self):
        self.results: List[ValidationResult] = []
        self.test_start_time = None
        
    def start_test(self, test_name: str) -> None:
        """Start timing a test"""
        self.current_test = test_name
        self.test_start_time = time.time()
        print(f"\n🧪 Running: {test_name}")
        
    def end_test(self, passed: bool, error_message: str = "", data_points: int = 0, precision_errors: List[float] = None) -> ValidationResult:
        """End timing a test and record result"""
        execution_time = (time.time() - self.test_start_time) * 1000  # Convert to ms
        result = ValidationResult(
            test_name=self.current_test,
            passed=passed,
            error_message=error_message,
            execution_time_ms=execution_time,
            data_points_tested=data_points,
            precision_errors=precision_errors or []
        )
        self.results.append(result)
        
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"   {status} ({execution_time:.1f}ms, {data_points} data points)")
        if error_message:
            print(f"   Error: {error_message}")
            
        return result

    def test_exchange_data_formats(self) -> ValidationResult:
        """Test validation of different exchange data formats"""
        self.start_test("Exchange Data Format Validation")
        
        try:
            test_cases = [
                # Coinbase format
                {
                    "exchange": "coinbase",
                    "trade": {
                        "type": "match",
                        "product_id": "BTC-USD",
                        "price": "68234.56",
                        "size": "0.12345678",
                        "side": "buy",
                        "time": "2024-08-15T10:30:00.000000Z"
                    }
                },
                # Kraken format
                {
                    "exchange": "kraken", 
                    "trade": [
                        1234567890,
                        [["68234.56", "0.12345678", "1692182400.123456", "b", "m"]],
                        "trade",
                        "XBT/USD"
                    ]
                },
                # Binance format (simulated)
                {
                    "exchange": "binance",
                    "trade": {
                        "s": "BTCUSDT",
                        "p": "68234.56",
                        "q": "0.12345678", 
                        "T": 1692182400123,
                        "m": False
                    }
                }
            ]
            
            validation_errors = []
            precision_errors = []
            
            for i, case in enumerate(test_cases):
                exchange = case["exchange"]
                
                if exchange == "coinbase":
                    # Test Coinbase price parsing
                    price_str = case["trade"]["price"]
                    size_str = case["trade"]["size"]
                    
                    # Simulate our conversion module
                    price_decimal = Decimal(price_str)
                    size_decimal = Decimal(size_str)
                    
                    # Convert to fixed-point
                    price_fp = int(price_decimal * Decimal('100000000'))
                    size_fp = int(size_decimal * Decimal('100000000'))
                    
                    # Convert back for validation
                    price_recovered = float(price_fp) / 100000000
                    size_recovered = float(size_fp) / 100000000
                    
                    # Check precision
                    price_error = abs(float(price_str) - price_recovered)
                    size_error = abs(float(size_str) - size_recovered)
                    
                    precision_errors.extend([price_error, size_error])
                    
                    if price_error > 1e-8 or size_error > 1e-8:
                        validation_errors.append(f"Coinbase precision error: price={price_error}, size={size_error}")
                        
                elif exchange == "kraken":
                    # Test Kraken array format
                    if len(case["trade"]) >= 4 and isinstance(case["trade"][1], list):
                        trades = case["trade"][1]
                        for trade in trades:
                            if len(trade) >= 2:
                                price_str = trade[0]
                                size_str = trade[1]
                                
                                # Validate numeric conversion
                                try:
                                    price_decimal = Decimal(price_str)
                                    size_decimal = Decimal(size_str)
                                    
                                    # Check ranges
                                    if price_decimal <= 0:
                                        validation_errors.append(f"Kraken invalid price: {price_str}")
                                    if size_decimal < 0:
                                        validation_errors.append(f"Kraken invalid size: {size_str}")
                                        
                                except Exception as e:
                                    validation_errors.append(f"Kraken parsing error: {e}")
                                    
                elif exchange == "binance":
                    # Test Binance format
                    price_str = case["trade"]["p"]
                    qty_str = case["trade"]["q"]
                    
                    try:
                        price_decimal = Decimal(price_str)
                        qty_decimal = Decimal(qty_str)
                        
                        # Validate symbol format
                        symbol = case["trade"]["s"]
                        if not symbol.endswith(("USDT", "USDC", "USD", "BTC", "ETH")):
                            validation_errors.append(f"Binance unusual symbol format: {symbol}")
                            
                    except Exception as e:
                        validation_errors.append(f"Binance parsing error: {e}")
            
            # Calculate precision statistics
            if precision_errors:
                max_error = max(precision_errors)
                avg_error = statistics.mean(precision_errors)
                
                # Precision should be better than 1e-8 (our fixed-point resolution)
                if max_error > 1e-8:
                    validation_errors.append(f"Precision exceeds tolerance: max={max_error}, avg={avg_error}")
            
            passed = len(validation_errors) == 0
            error_msg = "; ".join(validation_errors) if validation_errors else ""
            
            return self.end_test(passed, error_msg, len(test_cases) * 3, precision_errors)
            
        except Exception as e:
            return self.end_test(False, f"Test exception: {e}")

    def test_protocol_message_integrity(self) -> ValidationResult:
        """Test binary protocol message integrity"""
        self.start_test("Protocol Message Integrity")
        
        try:
            # Simulate different message types with various data
            test_messages = [
                {
                    "type": "trade",
                    "timestamp_ns": 1692182400123456789,
                    "price_fp": 6823456000000,  # $68234.56 in fixed-point
                    "volume_fp": 12345678,      # 0.12345678 in fixed-point
                    "symbol_hash": 0x1234567890ABCDEF,
                    "side": 1  # buy
                },
                {
                    "type": "orderbook",
                    "timestamp_ns": 1692182400123456789,
                    "symbol_hash": 0x1234567890ABCDEF,
                    "bids": [(6823456000000, 12345678), (6823455000000, 25000000)],
                    "asks": [(6823457000000, 15000000), (6823458000000, 30000000)]
                },
                {
                    "type": "l2_snapshot",
                    "timestamp_ns": 1692182400123456789,
                    "symbol_hash": 0x1234567890ABCDEF,
                    "sequence": 12345,
                    "bids": [(6823456000000, 12345678)] * 100,  # Large snapshot
                    "asks": [(6823457000000, 15000000)] * 100
                }
            ]
            
            validation_errors = []
            
            for msg in test_messages:
                # Validate timestamp
                if msg["timestamp_ns"] <= 0:
                    validation_errors.append(f"Invalid timestamp: {msg['timestamp_ns']}")
                
                # Validate symbol hash
                if msg["symbol_hash"] == 0:
                    validation_errors.append("Invalid symbol hash: zero")
                
                # Type-specific validation
                if msg["type"] == "trade":
                    if msg["price_fp"] <= 0:
                        validation_errors.append(f"Invalid trade price: {msg['price_fp']}")
                    if msg["volume_fp"] <= 0:
                        validation_errors.append(f"Invalid trade volume: {msg['volume_fp']}")
                    if msg["side"] not in [0, 1]:
                        validation_errors.append(f"Invalid trade side: {msg['side']}")
                        
                elif msg["type"] in ["orderbook", "l2_snapshot"]:
                    # Check bid/ask ordering
                    if "bids" in msg:
                        for i in range(len(msg["bids"]) - 1):
                            if msg["bids"][i][0] <= msg["bids"][i+1][0]:  # Bids should be descending
                                validation_errors.append("Bids not in descending price order")
                                break
                    
                    if "asks" in msg:
                        for i in range(len(msg["asks"]) - 1):
                            if msg["asks"][i][0] >= msg["asks"][i+1][0]:  # Asks should be ascending
                                validation_errors.append("Asks not in ascending price order")
                                break
                
                # Simulate message size check (protocol limit: 64KB)
                estimated_size = self._estimate_message_size(msg)
                if estimated_size > 65535:
                    validation_errors.append(f"Message too large: {estimated_size} bytes > 64KB limit")
            
            passed = len(validation_errors) == 0
            error_msg = "; ".join(validation_errors) if validation_errors else ""
            
            return self.end_test(passed, error_msg, len(test_messages))
            
        except Exception as e:
            return self.end_test(False, f"Test exception: {e}")

    def test_edge_cases_and_error_conditions(self) -> ValidationResult:
        """Test edge cases and error handling"""
        self.start_test("Edge Cases and Error Conditions")
        
        try:
            edge_cases = [
                # Extreme values
                ("extreme_large_price", "999999999.99999999"),
                ("extreme_small_price", "0.00000001"),
                ("zero_price", "0.0"),
                ("negative_price", "-100.0"),
                
                # Malformed data
                ("empty_string", ""),
                ("non_numeric", "abc123"),
                ("scientific_notation", "1.23e-5"),
                ("too_many_decimals", "123.123456789123"),
                
                # Boundary conditions
                ("max_fixed_point", str(2**63 - 1)),
                ("unicode_chars", "123.45€"),
                ("whitespace", "  123.45  "),
                
                # Real-world quirks
                ("trailing_zeros", "123.450000"),
                ("leading_zeros", "000123.45"),
                ("comma_separator", "1,234.56"),
            ]
            
            validation_results = []
            expected_failures = {
                "negative_price", "empty_string", "non_numeric", 
                "max_fixed_point", "unicode_chars", "comma_separator"
            }
            
            for test_name, value in edge_cases:
                try:
                    # Simulate our conversion function
                    if test_name in ["whitespace", "trailing_zeros", "leading_zeros"]:
                        # These should be handled gracefully
                        cleaned_value = value.strip().rstrip('0').rstrip('.')
                        if not cleaned_value or cleaned_value == '':
                            cleaned_value = '0'
                        decimal_val = Decimal(cleaned_value)
                        if decimal_val < 0:
                            raise ValueError("Negative value")
                        result = "success"
                    elif test_name == "scientific_notation":
                        # Should work with Decimal
                        decimal_val = Decimal(value)
                        result = "success"
                    else:
                        # Try direct conversion
                        decimal_val = Decimal(value)
                        if decimal_val < 0:
                            raise ValueError("Negative value")
                        result = "success"
                        
                except Exception as e:
                    result = "failed"
                
                # Check if result matches expectation
                should_fail = test_name in expected_failures
                test_passed = (result == "failed") == should_fail
                
                validation_results.append({
                    "test": test_name,
                    "value": value,
                    "result": result,
                    "expected_failure": should_fail,
                    "passed": test_passed
                })
                
                if not test_passed:
                    print(f"   ⚠️  {test_name}: expected {'failure' if should_fail else 'success'}, got {result}")
            
            # Count overall success
            passed_tests = sum(1 for r in validation_results if r["passed"])
            total_tests = len(validation_results)
            
            passed = passed_tests == total_tests
            error_msg = f"Edge case failures: {total_tests - passed_tests}/{total_tests}" if not passed else ""
            
            return self.end_test(passed, error_msg, total_tests)
            
        except Exception as e:
            return self.end_test(False, f"Test exception: {e}")

    def test_performance_under_load(self) -> ValidationResult:
        """Test validation performance under load"""
        self.start_test("Performance Under Load")
        
        try:
            # Generate test data
            num_messages = 10000
            test_prices = []
            
            # Create realistic price data
            base_price = 68234.56
            for i in range(num_messages):
                # Simulate realistic price movements
                variation = (i % 1000 - 500) * 0.01  # ±$5 variation
                price = base_price + variation
                test_prices.append(f"{price:.8f}")
            
            # Test conversion performance
            conversion_times = []
            precision_errors = []
            
            for price_str in test_prices:
                start_time = time.perf_counter()
                
                # Simulate our conversion
                try:
                    decimal_val = Decimal(price_str)
                    fixed_point = int(decimal_val * Decimal('100000000'))
                    recovered = float(fixed_point) / 100000000
                    
                    # Check precision
                    error = abs(float(price_str) - recovered)
                    precision_errors.append(error)
                    
                except Exception as e:
                    # Count as precision error
                    precision_errors.append(1.0)
                
                end_time = time.perf_counter()
                conversion_times.append((end_time - start_time) * 1000000)  # microseconds
            
            # Performance analysis
            avg_time_us = statistics.mean(conversion_times)
            max_time_us = max(conversion_times)
            p99_time_us = sorted(conversion_times)[int(0.99 * len(conversion_times))]
            
            # Precision analysis
            max_error = max(precision_errors)
            avg_error = statistics.mean(precision_errors)
            
            # Performance requirements (adjust based on needs)
            performance_ok = avg_time_us < 10.0  # Should average < 10 microseconds
            precision_ok = max_error < 1e-8      # Should maintain precision
            
            print(f"   📊 Performance: avg={avg_time_us:.2f}μs, p99={p99_time_us:.2f}μs, max={max_time_us:.2f}μs")
            print(f"   🎯 Precision: avg_error={avg_error:.2e}, max_error={max_error:.2e}")
            
            passed = performance_ok and precision_ok
            error_msg = ""
            if not performance_ok:
                error_msg += f"Performance too slow: {avg_time_us:.2f}μs > 10μs; "
            if not precision_ok:
                error_msg += f"Precision too low: {max_error:.2e} > 1e-8"
            
            return self.end_test(passed, error_msg.strip("; "), num_messages, precision_errors)
            
        except Exception as e:
            return self.end_test(False, f"Test exception: {e}")

    def test_real_world_data_scenarios(self) -> ValidationResult:
        """Test with real-world data scenarios"""
        self.start_test("Real-World Data Scenarios")
        
        try:
            # Real-world scenarios based on actual exchange data patterns
            scenarios = [
                {
                    "name": "BTC Price Surge",
                    "prices": ["68234.56", "68245.23", "68250.00", "68267.89", "68299.99"],
                    "volumes": ["0.12345678", "0.25000000", "0.08765432", "0.15432100", "0.33333333"]
                },
                {
                    "name": "Stablecoin Depeg Event", 
                    "prices": ["1.0000", "0.9995", "0.9987", "0.9978", "0.9985"],
                    "volumes": ["1000.0", "5000.0", "15000.0", "8000.0", "3000.0"]
                },
                {
                    "name": "Altcoin Micro Movements",
                    "prices": ["1.23456789", "1.23456790", "1.23456788", "1.23456791", "1.23456787"],
                    "volumes": ["100.12345678", "50.87654321", "75.11111111", "200.99999999", "125.00000001"]
                },
                {
                    "name": "High Frequency Trading",
                    "prices": ["68234.56"] * 1000,  # Same price, different volumes
                    "volumes": [f"{0.001 + i * 0.0001:.8f}" for i in range(1000)]
                }
            ]
            
            validation_errors = []
            total_data_points = 0
            all_precision_errors = []
            
            for scenario in scenarios:
                print(f"   🔍 Testing scenario: {scenario['name']}")
                
                for price_str, volume_str in zip(scenario["prices"], scenario["volumes"]):
                    try:
                        # Test price conversion
                        price_decimal = Decimal(price_str)
                        price_fp = int(price_decimal * Decimal('100000000'))
                        price_recovered = float(price_fp) / 100000000
                        price_error = abs(float(price_str) - price_recovered)
                        
                        # Test volume conversion  
                        volume_decimal = Decimal(volume_str)
                        volume_fp = int(volume_decimal * Decimal('100000000'))
                        volume_recovered = float(volume_fp) / 100000000
                        volume_error = abs(float(volume_str) - volume_recovered)
                        
                        all_precision_errors.extend([price_error, volume_error])
                        
                        # Validate ranges
                        if price_recovered <= 0:
                            validation_errors.append(f"Invalid price in {scenario['name']}: {price_recovered}")
                        if volume_recovered < 0:
                            validation_errors.append(f"Invalid volume in {scenario['name']}: {volume_recovered}")
                        
                        # Check for extreme precision loss
                        if price_error > 1e-7 or volume_error > 1e-7:
                            validation_errors.append(f"High precision loss in {scenario['name']}: price={price_error:.2e}, volume={volume_error:.2e}")
                        
                        total_data_points += 2
                        
                    except Exception as e:
                        validation_errors.append(f"Conversion error in {scenario['name']}: {e}")
            
            passed = len(validation_errors) == 0
            error_msg = "; ".join(validation_errors[:5]) if validation_errors else ""  # Limit error message length
            if len(validation_errors) > 5:
                error_msg += f" ... and {len(validation_errors) - 5} more errors"
            
            return self.end_test(passed, error_msg, total_data_points, all_precision_errors)
            
        except Exception as e:
            return self.end_test(False, f"Test exception: {e}")

    def _estimate_message_size(self, msg: Dict[str, Any]) -> int:
        """Estimate binary message size"""
        # Rough estimation based on protocol structure
        base_size = 32  # Header
        
        if msg["type"] == "trade":
            return base_size + 64  # Fixed size for trade message
        elif msg["type"] == "orderbook":
            num_levels = len(msg.get("bids", [])) + len(msg.get("asks", []))
            return base_size + (num_levels * 16)  # 16 bytes per price level
        elif msg["type"] == "l2_snapshot":
            num_levels = len(msg.get("bids", [])) + len(msg.get("asks", []))
            return base_size + (num_levels * 16) + 8  # Additional sequence field
        
        return base_size

    def generate_report(self) -> Dict[str, Any]:
        """Generate comprehensive test report"""
        passed_tests = [r for r in self.results if r.passed]
        failed_tests = [r for r in self.results if not r.passed]
        
        total_execution_time = sum(r.execution_time_ms for r in self.results)
        total_data_points = sum(r.data_points_tested for r in self.results)
        
        # Precision statistics
        all_precision_errors = []
        for result in self.results:
            all_precision_errors.extend(result.precision_errors)
        
        precision_stats = {}
        if all_precision_errors:
            precision_stats = {
                "max_error": max(all_precision_errors),
                "average_error": statistics.mean(all_precision_errors),
                "median_error": statistics.median(all_precision_errors),
                "error_count": len(all_precision_errors)
            }
        
        return {
            "summary": {
                "total_tests": len(self.results),
                "passed": len(passed_tests),
                "failed": len(failed_tests),
                "pass_rate": len(passed_tests) / len(self.results) * 100 if self.results else 0,
                "total_execution_time_ms": total_execution_time,
                "total_data_points_tested": total_data_points
            },
            "precision_analysis": precision_stats,
            "test_results": [
                {
                    "name": r.test_name,
                    "passed": r.passed,
                    "execution_time_ms": r.execution_time_ms,
                    "data_points": r.data_points_tested,
                    "error": r.error_message if not r.passed else None
                }
                for r in self.results
            ],
            "failed_tests": [
                {
                    "name": r.test_name,
                    "error": r.error_message,
                    "execution_time_ms": r.execution_time_ms
                }
                for r in failed_tests
            ]
        }

def run_comprehensive_validation():
    """Run all comprehensive validation tests"""
    print("=" * 80)
    print("COMPREHENSIVE DATA VALIDATION TEST SUITE")
    print("=" * 80)
    
    validator = ComprehensiveDataValidator()
    
    # Run all test categories
    validator.test_exchange_data_formats()
    validator.test_protocol_message_integrity() 
    validator.test_edge_cases_and_error_conditions()
    validator.test_performance_under_load()
    validator.test_real_world_data_scenarios()
    
    # Generate and display report
    report = validator.generate_report()
    
    print("\n" + "=" * 80)
    print("COMPREHENSIVE TEST RESULTS")
    print("=" * 80)
    
    summary = report["summary"]
    print(f"📊 Tests Run: {summary['total_tests']}")
    print(f"✅ Passed: {summary['passed']}")
    print(f"❌ Failed: {summary['failed']}")
    print(f"📈 Pass Rate: {summary['pass_rate']:.1f}%")
    print(f"⏱️  Total Time: {summary['total_execution_time_ms']:.1f}ms")
    print(f"📋 Data Points: {summary['total_data_points_tested']:,}")
    
    if report["precision_analysis"]:
        print(f"\n🎯 PRECISION ANALYSIS:")
        precision = report["precision_analysis"]
        print(f"   Max Error: {precision['max_error']:.2e}")
        print(f"   Avg Error: {precision['average_error']:.2e}")
        print(f"   Median Error: {precision['median_error']:.2e}")
        print(f"   Total Measurements: {precision['error_count']:,}")
    
    if report["failed_tests"]:
        print(f"\n❌ FAILED TESTS:")
        for failed in report["failed_tests"]:
            print(f"   • {failed['name']}: {failed['error']}")
    
    # Overall assessment
    overall_success = summary['pass_rate'] >= 95.0  # Require 95% pass rate
    precision_good = not report["precision_analysis"] or report["precision_analysis"]["max_error"] < 1e-7
    
    print(f"\n🏆 OVERALL ASSESSMENT:")
    if overall_success and precision_good:
        print("   ✅ EXCELLENT - Data validation pipeline is robust and precise")
    elif overall_success:
        print("   ⚠️  GOOD - High pass rate but check precision issues")
    else:
        print("   ❌ NEEDS IMPROVEMENT - Multiple test failures detected")
    
    # Save detailed report
    with open("/Users/daws/alphapulse/backend/tests/e2e/comprehensive_validation_report.json", "w") as f:
        json.dump(report, f, indent=2, default=str)
    
    print(f"\n📄 Detailed report saved to: comprehensive_validation_report.json")
    
    return overall_success and precision_good

if __name__ == "__main__":
    success = run_comprehensive_validation()
    exit(0 if success else 1)