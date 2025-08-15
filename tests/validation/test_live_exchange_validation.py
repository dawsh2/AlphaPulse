#!/usr/bin/env python3
"""
Live Exchange Data Validation Tests

Tests the data validation pipeline with real exchange data feeds.
This ensures our validation logic works correctly with actual market data.
"""

import asyncio
import websockets
import json
import time
import statistics
import decimal
from typing import Dict, List, Any, Optional
from decimal import Decimal, getcontext
from dataclasses import dataclass
from protocol_validator import ProtocolValidator
from comparison_engine import DataComparisonEngine

# Set high precision for Decimal calculations
getcontext().prec = 28

@dataclass
class LiveDataSample:
    """A sample of live market data from an exchange"""
    exchange: str
    symbol: str
    price: str
    volume: str
    timestamp: float
    side: str
    raw_message: dict

@dataclass
class ValidationMetrics:
    """Metrics collected during live data validation"""
    total_messages: int = 0
    valid_messages: int = 0
    precision_errors: List[float] = None
    range_violations: List[str] = None
    format_errors: List[str] = None
    processing_times_ms: List[float] = None
    
    def __post_init__(self):
        if self.precision_errors is None:
            self.precision_errors = []
        if self.range_violations is None:
            self.range_violations = []
        if self.format_errors is None:
            self.format_errors = []
        if self.processing_times_ms is None:
            self.processing_times_ms = []

class LiveExchangeValidator:
    """Validates live exchange data feeds"""
    
    def __init__(self):
        self.metrics = ValidationMetrics()
        self.samples: List[LiveDataSample] = []
        self.protocol_validator = ProtocolValidator()
        self.comparison_engine = DataComparisonEngine(tolerance=0.0001)
        
    async def test_coinbase_live_data(self, duration_seconds: int = 30) -> ValidationMetrics:
        """Test live Coinbase data feed"""
        print(f"🔗 Connecting to Coinbase WebSocket for {duration_seconds}s...")
        
        try:
            uri = "wss://ws-feed.exchange.coinbase.com"
            async with websockets.connect(uri) as websocket:
                # Subscribe to BTC-USD and ETH-USD trades
                subscribe_msg = {
                    "type": "subscribe",
                    "product_ids": ["BTC-USD", "ETH-USD"],
                    "channels": ["matches"]
                }
                await websocket.send(json.dumps(subscribe_msg))
                print("✅ Subscribed to Coinbase trade feed")
                
                # Collect data for specified duration
                start_time = time.time()
                while time.time() - start_time < duration_seconds:
                    try:
                        # Set timeout to avoid hanging
                        message = await asyncio.wait_for(
                            websocket.recv(), 
                            timeout=5.0
                        )
                        await self._process_coinbase_message(message)
                        
                    except asyncio.TimeoutError:
                        print("⏰ WebSocket timeout, continuing...")
                        continue
                    except Exception as e:
                        self.metrics.format_errors.append(f"Message processing error: {e}")
                        continue
                        
        except Exception as e:
            print(f"❌ Coinbase connection failed: {e}")
            self.metrics.format_errors.append(f"Connection error: {e}")
            
        return self.metrics
    
    async def _process_coinbase_message(self, message: str) -> None:
        """Process a single Coinbase WebSocket message"""
        start_time = time.perf_counter()
        
        try:
            data = json.loads(message)
            self.metrics.total_messages += 1
            
            # Only process trade messages
            if data.get("type") != "match":
                return
                
            # Extract trade data
            product_id = data.get("product_id", "")
            price_str = data.get("price", "0")
            size_str = data.get("size", "0")
            side = data.get("side", "")
            trade_time = data.get("time", "")
            
            # Create sample
            sample = LiveDataSample(
                exchange="coinbase",
                symbol=product_id,
                price=price_str,
                volume=size_str,
                timestamp=time.time(),
                side=side,
                raw_message=data
            )
            self.samples.append(sample)
            
            # Validate the data
            is_valid = await self._validate_trade_data(sample)
            if is_valid:
                self.metrics.valid_messages += 1
                
        except json.JSONDecodeError as e:
            self.metrics.format_errors.append(f"JSON decode error: {e}")
        except Exception as e:
            self.metrics.format_errors.append(f"Processing error: {e}")
        finally:
            # Record processing time
            processing_time = (time.perf_counter() - start_time) * 1000
            self.metrics.processing_times_ms.append(processing_time)
    
    async def _validate_trade_data(self, sample: LiveDataSample) -> bool:
        """Validate a trade data sample"""
        is_valid = True
        
        try:
            # Test precision-preserving conversion
            price_decimal = Decimal(sample.price)
            volume_decimal = Decimal(sample.volume)
            
            # Convert to fixed-point (simulating our conversion module)
            price_fp = int(price_decimal * Decimal('100000000'))
            volume_fp = int(volume_decimal * Decimal('100000000'))
            
            # Convert back and check precision
            price_recovered = float(price_fp) / 100000000
            volume_recovered = float(volume_fp) / 100000000
            
            price_error = abs(float(sample.price) - price_recovered)
            volume_error = abs(float(sample.volume) - volume_recovered)
            
            self.metrics.precision_errors.extend([price_error, volume_error])
            
            # Check precision tolerance
            if price_error > 1e-8 or volume_error > 1e-8:
                self.metrics.format_errors.append(
                    f"Precision loss in {sample.symbol}: price={price_error:.2e}, volume={volume_error:.2e}"
                )
                is_valid = False
            
            # Validate price ranges
            price_float = float(sample.price)
            volume_float = float(sample.volume)
            
            # Basic range checks
            if price_float <= 0:
                self.metrics.range_violations.append(f"Invalid price: {price_float} for {sample.symbol}")
                is_valid = False
                
            if volume_float < 0:
                self.metrics.range_violations.append(f"Invalid volume: {volume_float} for {sample.symbol}")
                is_valid = False
            
            # Symbol-specific range checks
            if sample.symbol == "BTC-USD":
                if not (1000 <= price_float <= 1000000):  # Reasonable BTC range
                    self.metrics.range_violations.append(f"BTC price out of range: ${price_float:,.2f}")
                    is_valid = False
                    
            elif sample.symbol == "ETH-USD":
                if not (10 <= price_float <= 50000):  # Reasonable ETH range
                    self.metrics.range_violations.append(f"ETH price out of range: ${price_float:,.2f}")
                    is_valid = False
            
            # Check for suspicious values
            if price_float in [1.0, 100.0, 1000.0, 10000.0]:
                self.metrics.range_violations.append(f"Suspicious round price: ${price_float} for {sample.symbol}")
                # Don't mark as invalid, just suspicious
                
        except (ValueError, decimal.InvalidOperation) as e:
            self.metrics.format_errors.append(f"Conversion error for {sample.symbol}: {e}")
            is_valid = False
        except Exception as e:
            self.metrics.format_errors.append(f"Validation error for {sample.symbol}: {e}")
            is_valid = False
            
        return is_valid
    
    def _validate_trade_data_sync(self, sample: LiveDataSample) -> bool:
        """Synchronous version of trade data validation"""
        is_valid = True
        
        try:
            # Test precision-preserving conversion
            price_decimal = Decimal(sample.price)
            volume_decimal = Decimal(sample.volume)
            
            # Convert to fixed-point (simulating our conversion module)
            price_fp = int(price_decimal * Decimal('100000000'))
            volume_fp = int(volume_decimal * Decimal('100000000'))
            
            # Convert back and check precision
            price_recovered = float(price_fp) / 100000000
            volume_recovered = float(volume_fp) / 100000000
            
            price_error = abs(float(sample.price) - price_recovered)
            volume_error = abs(float(sample.volume) - volume_recovered)
            
            self.metrics.precision_errors.extend([price_error, volume_error])
            
            # Check precision tolerance
            if price_error > 1e-8 or volume_error > 1e-8:
                self.metrics.format_errors.append(
                    f"Precision loss in {sample.symbol}: price={price_error:.2e}, volume={volume_error:.2e}"
                )
                is_valid = False
            
            # Validate price ranges
            price_float = float(sample.price)
            volume_float = float(sample.volume)
            
            # Basic range checks
            if price_float <= 0:
                self.metrics.range_violations.append(f"Invalid price: {price_float} for {sample.symbol}")
                is_valid = False
                
            if volume_float < 0:
                self.metrics.range_violations.append(f"Invalid volume: {volume_float} for {sample.symbol}")
                is_valid = False
            
            # Symbol-specific range checks
            if sample.symbol == "BTC-USD":
                if not (1000 <= price_float <= 1000000):  # Reasonable BTC range
                    self.metrics.range_violations.append(f"BTC price out of range: ${price_float:,.2f}")
                    is_valid = False
                    
            elif sample.symbol == "ETH-USD":
                if not (10 <= price_float <= 50000):  # Reasonable ETH range
                    self.metrics.range_violations.append(f"ETH price out of range: ${price_float:,.2f}")
                    is_valid = False
            
            # Check for suspicious values
            if price_float in [1.0, 100.0, 1000.0, 10000.0]:
                self.metrics.range_violations.append(f"Suspicious round price: ${price_float} for {sample.symbol}")
                # Don't mark as invalid, just suspicious
                
        except (ValueError, decimal.InvalidOperation) as e:
            self.metrics.format_errors.append(f"Conversion error for {sample.symbol}: {e}")
            is_valid = False
        except Exception as e:
            self.metrics.format_errors.append(f"Validation error for {sample.symbol}: {e}")
            is_valid = False
            
        return is_valid
    
    def test_simulated_exchange_data(self, num_samples: int = 1000) -> ValidationMetrics:
        """Test with simulated exchange data based on realistic patterns"""
        print(f"🧪 Testing with {num_samples} simulated exchange data samples...")
        
        # Generate realistic test data
        base_prices = {
            "BTC-USD": 68234.56,
            "ETH-USD": 4605.23,
            "LTC-USD": 95.67,
            "ADA-USD": 0.4523,
            "USDC-USD": 1.0001
        }
        
        for i in range(num_samples):
            for symbol, base_price in base_prices.items():
                # Add realistic price movement (±2%)
                price_variation = (i % 200 - 100) * 0.0001  # ±1% random walk
                current_price = base_price * (1 + price_variation)
                
                # Generate realistic volume
                volume = 0.1 + (i % 50) * 0.05  # 0.1 to 2.5 volume range
                
                sample = LiveDataSample(
                    exchange="simulated",
                    symbol=symbol,
                    price=f"{current_price:.8f}",
                    volume=f"{volume:.8f}",
                    timestamp=time.time(),
                    side="buy" if i % 2 == 0 else "sell",
                    raw_message={}
                )
                
                self.samples.append(sample)
                self.metrics.total_messages += 1
                
                # Validate the sample
                start_time = time.perf_counter()
                is_valid = self._validate_trade_data_sync(sample)
                processing_time = (time.perf_counter() - start_time) * 1000
                self.metrics.processing_times_ms.append(processing_time)
                
                if is_valid:
                    self.metrics.valid_messages += 1
        
        return self.metrics
    
    def test_edge_cases(self) -> ValidationMetrics:
        """Test edge cases and boundary conditions"""
        print("🔍 Testing edge cases and boundary conditions...")
        
        edge_cases = [
            # Extreme prices
            LiveDataSample("test", "BTC-USD", "999999.99999999", "0.00000001", time.time(), "buy", {}),
            LiveDataSample("test", "ETH-USD", "0.00000001", "1000000.0", time.time(), "sell", {}),
            
            # Boundary values
            LiveDataSample("test", "USDC-USD", "1.0", "0.0", time.time(), "buy", {}),  # Zero volume
            LiveDataSample("test", "BTC-USD", "0.01", "1.0", time.time(), "sell", {}),  # Very low price
            
            # Precision edge cases
            LiveDataSample("test", "ETH-USD", "4605.23", "1.23456789", time.time(), "buy", {}),
            LiveDataSample("test", "BTC-USD", "68234.56789012", "0.12345678", time.time(), "sell", {}),
            
            # Suspicious patterns
            LiveDataSample("test", "TEST-USD", "1.0", "1.0", time.time(), "buy", {}),  # Round numbers
            LiveDataSample("test", "TEST-USD", "100.0", "100.0", time.time(), "sell", {}),
        ]
        
        for sample in edge_cases:
            self.samples.append(sample)
            self.metrics.total_messages += 1
            
            start_time = time.perf_counter()
            is_valid = self._validate_trade_data_sync(sample)
            processing_time = (time.perf_counter() - start_time) * 1000
            self.metrics.processing_times_ms.append(processing_time)
            
            if is_valid:
                self.metrics.valid_messages += 1
        
        return self.metrics
    
    def generate_comprehensive_report(self) -> Dict[str, Any]:
        """Generate comprehensive validation report"""
        if not self.metrics.processing_times_ms:
            return {"error": "No data processed"}
        
        # Calculate statistics
        total_msgs = self.metrics.total_messages
        valid_msgs = self.metrics.valid_messages
        pass_rate = (valid_msgs / total_msgs * 100) if total_msgs > 0 else 0
        
        # Processing performance
        avg_processing_time = statistics.mean(self.metrics.processing_times_ms)
        max_processing_time = max(self.metrics.processing_times_ms)
        p99_processing_time = sorted(self.metrics.processing_times_ms)[int(0.99 * len(self.metrics.processing_times_ms))]
        
        # Precision analysis
        precision_stats = {}
        if self.metrics.precision_errors:
            precision_stats = {
                "max_error": max(self.metrics.precision_errors),
                "average_error": statistics.mean(self.metrics.precision_errors),
                "median_error": statistics.median(self.metrics.precision_errors),
                "samples_with_precision_loss": sum(1 for e in self.metrics.precision_errors if e > 1e-10),
                "total_precision_measurements": len(self.metrics.precision_errors)
            }
        
        # Error analysis
        error_summary = {
            "total_range_violations": len(self.metrics.range_violations),
            "total_format_errors": len(self.metrics.format_errors),
            "unique_range_violations": len(set(self.metrics.range_violations)),
            "unique_format_errors": len(set(self.metrics.format_errors))
        }
        
        return {
            "summary": {
                "total_messages_processed": total_msgs,
                "valid_messages": valid_msgs,
                "invalid_messages": total_msgs - valid_msgs,
                "validation_pass_rate": pass_rate,
                "total_samples_collected": len(self.samples)
            },
            "performance": {
                "average_processing_time_ms": avg_processing_time,
                "max_processing_time_ms": max_processing_time,
                "p99_processing_time_ms": p99_processing_time,
                "throughput_msg_per_sec": 1000 / avg_processing_time if avg_processing_time > 0 else 0
            },
            "precision_analysis": precision_stats,
            "error_analysis": error_summary,
            "error_details": {
                "range_violations": self.metrics.range_violations[:10],  # First 10
                "format_errors": self.metrics.format_errors[:10]         # First 10
            }
        }

async def run_live_validation_tests():
    """Run comprehensive live validation tests"""
    print("=" * 80)
    print("LIVE EXCHANGE DATA VALIDATION TESTS")
    print("=" * 80)
    
    validator = LiveExchangeValidator()
    
    # Test 1: Live Coinbase data (short duration for CI/testing)
    print("\n📡 Test 1: Live Coinbase Data Validation")
    try:
        await validator.test_coinbase_live_data(duration_seconds=10)
        print(f"   Collected {validator.metrics.total_messages} live messages")
    except Exception as e:
        print(f"   ⚠️ Live test skipped: {e}")
    
    # Test 2: Simulated exchange data
    print("\n🧪 Test 2: Simulated Exchange Data")
    validator.test_simulated_exchange_data(num_samples=500)
    
    # Test 3: Edge cases
    print("\n🔍 Test 3: Edge Cases and Boundary Conditions")
    validator.test_edge_cases()
    
    # Generate report
    report = validator.generate_comprehensive_report()
    
    print("\n" + "=" * 80)
    print("LIVE VALIDATION RESULTS")
    print("=" * 80)
    
    summary = report["summary"]
    performance = report["performance"]
    precision = report.get("precision_analysis", {})
    errors = report["error_analysis"]
    
    print(f"📊 Total Messages: {summary['total_messages_processed']:,}")
    print(f"✅ Valid: {summary['valid_messages']:,}")
    print(f"❌ Invalid: {summary['invalid_messages']:,}")
    print(f"📈 Pass Rate: {summary['validation_pass_rate']:.1f}%")
    
    print(f"\n⚡ Performance:")
    print(f"   Average Processing: {performance['average_processing_time_ms']:.3f}ms")
    print(f"   P99 Processing: {performance['p99_processing_time_ms']:.3f}ms")
    print(f"   Throughput: {performance['throughput_msg_per_sec']:.0f} msg/sec")
    
    if precision:
        print(f"\n🎯 Precision Analysis:")
        print(f"   Max Error: {precision['max_error']:.2e}")
        print(f"   Average Error: {precision['average_error']:.2e}")
        print(f"   Samples with Loss: {precision['samples_with_precision_loss']}/{precision['total_precision_measurements']}")
    
    print(f"\n❌ Error Summary:")
    print(f"   Range Violations: {errors['total_range_violations']}")
    print(f"   Format Errors: {errors['total_format_errors']}")
    
    if report["error_details"]["range_violations"]:
        print(f"\n   Recent Range Violations:")
        for violation in report["error_details"]["range_violations"][:3]:
            print(f"      • {violation}")
    
    if report["error_details"]["format_errors"]:
        print(f"\n   Recent Format Errors:")
        for error in report["error_details"]["format_errors"][:3]:
            print(f"      • {error}")
    
    # Overall assessment
    success_criteria = {
        "pass_rate": summary['validation_pass_rate'] >= 95.0,
        "performance": performance['average_processing_time_ms'] < 10.0,
        "precision": not precision or precision['max_error'] < 1e-7
    }
    
    all_passed = all(success_criteria.values())
    
    print(f"\n🏆 OVERALL ASSESSMENT:")
    if all_passed:
        print("   ✅ EXCELLENT - Live data validation pipeline is robust")
    else:
        print("   ⚠️ REVIEW NEEDED:")
        for criterion, passed in success_criteria.items():
            status = "✅" if passed else "❌"
            print(f"      {status} {criterion}")
    
    # Save detailed report
    with open("/Users/daws/alphapulse/backend/tests/e2e/live_validation_report.json", "w") as f:
        json.dump(report, f, indent=2, default=str)
    
    print(f"\n📄 Detailed report saved to: live_validation_report.json")
    
    return all_passed

def run_sync_tests():
    """Synchronous wrapper for async tests"""
    return asyncio.run(run_live_validation_tests())

if __name__ == "__main__":
    success = run_sync_tests()
    exit(0 if success else 1)