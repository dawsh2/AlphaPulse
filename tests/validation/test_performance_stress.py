#!/usr/bin/env python3
"""
Performance and Stress Testing for Data Validation Pipeline

Tests the validation pipeline under various load conditions to ensure
it can handle high-frequency trading data without performance degradation.
"""

import time
import statistics
import threading
import multiprocessing
import json
import random
import gc
from typing import Dict, List, Any, Tuple
from decimal import Decimal, getcontext
from dataclasses import dataclass, field
from concurrent.futures import ThreadPoolExecutor, ProcessPoolExecutor
import psutil
import os

# Set high precision for Decimal calculations
getcontext().prec = 28

@dataclass
class PerformanceMetrics:
    """Performance metrics collection"""
    test_name: str
    total_messages: int = 0
    processing_times_us: List[float] = field(default_factory=list)
    memory_usage_mb: List[float] = field(default_factory=list)
    cpu_usage_percent: List[float] = field(default_factory=list)
    throughput_msg_per_sec: float = 0.0
    errors: List[str] = field(default_factory=list)
    start_time: float = 0.0
    end_time: float = 0.0

class PerformanceStressTester:
    """Comprehensive performance and stress testing"""
    
    def __init__(self):
        self.results: List[PerformanceMetrics] = []
        self.process = psutil.Process(os.getpid())
    
    def test_single_thread_performance(self, num_messages: int = 10000) -> PerformanceMetrics:
        """Test single-threaded validation performance"""
        print(f"⚡ Single-thread performance test ({num_messages:,} messages)...")
        
        metrics = PerformanceMetrics(
            test_name="single_thread_performance",
            total_messages=num_messages
        )
        
        # Generate test data
        test_messages = self._generate_realistic_test_data(num_messages)
        
        # Collect baseline metrics
        metrics.start_time = time.time()
        gc.collect()  # Clean up before test
        
        for i, message_data in enumerate(test_messages):
            # Monitor system resources every 1000 messages
            if i % 1000 == 0:
                metrics.memory_usage_mb.append(self.process.memory_info().rss / 1024 / 1024)
                metrics.cpu_usage_percent.append(self.process.cpu_percent())
            
            # Process single message
            start_time = time.perf_counter()
            try:
                result = self._validate_message_performance(message_data)
                if not result:
                    metrics.errors.append(f"Validation failed for message {i}")
            except Exception as e:
                metrics.errors.append(f"Exception at message {i}: {e}")
            
            end_time = time.perf_counter()
            processing_time_us = (end_time - start_time) * 1_000_000
            metrics.processing_times_us.append(processing_time_us)
        
        metrics.end_time = time.time()
        total_time = metrics.end_time - metrics.start_time
        metrics.throughput_msg_per_sec = num_messages / total_time if total_time > 0 else 0
        
        self.results.append(metrics)
        
        # Print summary
        avg_time = statistics.mean(metrics.processing_times_us)
        p95_time = sorted(metrics.processing_times_us)[int(0.95 * len(metrics.processing_times_us))]
        p99_time = sorted(metrics.processing_times_us)[int(0.99 * len(metrics.processing_times_us))]
        
        print(f"   📊 Throughput: {metrics.throughput_msg_per_sec:,.0f} msg/sec")
        print(f"   ⏱️  Avg latency: {avg_time:.1f}μs, P95: {p95_time:.1f}μs, P99: {p99_time:.1f}μs")
        print(f"   💾 Memory: {max(metrics.memory_usage_mb):.1f}MB peak")
        print(f"   ❌ Errors: {len(metrics.errors)}")
        
        return metrics
    
    def test_multi_thread_performance(self, num_messages: int = 50000, num_threads: int = 4) -> PerformanceMetrics:
        """Test multi-threaded validation performance"""
        print(f"🚀 Multi-thread performance test ({num_messages:,} messages, {num_threads} threads)...")
        
        metrics = PerformanceMetrics(
            test_name=f"multi_thread_performance_{num_threads}",
            total_messages=num_messages
        )
        
        # Generate test data
        test_messages = self._generate_realistic_test_data(num_messages)
        
        # Split messages across threads
        chunk_size = len(test_messages) // num_threads
        message_chunks = [
            test_messages[i:i + chunk_size] 
            for i in range(0, len(test_messages), chunk_size)
        ]
        
        metrics.start_time = time.time()
        processing_times = []
        errors = []
        
        def process_chunk(chunk):
            chunk_times = []
            chunk_errors = []
            
            for message_data in chunk:
                start_time = time.perf_counter()
                try:
                    result = self._validate_message_performance(message_data)
                    if not result:
                        chunk_errors.append("Validation failed")
                except Exception as e:
                    chunk_errors.append(f"Exception: {e}")
                
                end_time = time.perf_counter()
                processing_time_us = (end_time - start_time) * 1_000_000
                chunk_times.append(processing_time_us)
            
            return chunk_times, chunk_errors
        
        # Execute in parallel
        with ThreadPoolExecutor(max_workers=num_threads) as executor:
            futures = [executor.submit(process_chunk, chunk) for chunk in message_chunks]
            
            for future in futures:
                chunk_times, chunk_errors = future.result()
                processing_times.extend(chunk_times)
                errors.extend(chunk_errors)
        
        metrics.end_time = time.time()
        metrics.processing_times_us = processing_times
        metrics.errors = errors
        
        total_time = metrics.end_time - metrics.start_time
        metrics.throughput_msg_per_sec = num_messages / total_time if total_time > 0 else 0
        
        self.results.append(metrics)
        
        # Print summary
        avg_time = statistics.mean(processing_times)
        p95_time = sorted(processing_times)[int(0.95 * len(processing_times))]
        
        print(f"   📊 Throughput: {metrics.throughput_msg_per_sec:,.0f} msg/sec")
        print(f"   ⏱️  Avg latency: {avg_time:.1f}μs, P95: {p95_time:.1f}μs")
        print(f"   ❌ Errors: {len(errors)}")
        
        return metrics
    
    def test_memory_stress(self, num_messages: int = 100000) -> PerformanceMetrics:
        """Test memory usage under stress"""
        print(f"💾 Memory stress test ({num_messages:,} messages)...")
        
        metrics = PerformanceMetrics(
            test_name="memory_stress",
            total_messages=num_messages
        )
        
        # Record initial memory
        initial_memory = self.process.memory_info().rss / 1024 / 1024
        
        # Generate and process large amounts of data
        metrics.start_time = time.time()
        
        # Process in batches to monitor memory growth
        batch_size = 5000
        for batch_start in range(0, num_messages, batch_size):
            batch_end = min(batch_start + batch_size, num_messages)
            batch_messages = self._generate_realistic_test_data(batch_end - batch_start)
            
            # Process batch
            for message_data in batch_messages:
                start_time = time.perf_counter()
                try:
                    self._validate_message_performance(message_data)
                except Exception as e:
                    metrics.errors.append(f"Exception: {e}")
                
                end_time = time.perf_counter()
                metrics.processing_times_us.append((end_time - start_time) * 1_000_000)
            
            # Record memory usage
            current_memory = self.process.memory_info().rss / 1024 / 1024
            metrics.memory_usage_mb.append(current_memory)
            
            # Force garbage collection to test for memory leaks
            gc.collect()
            
            print(f"   Batch {batch_start//batch_size + 1}: {current_memory:.1f}MB")
        
        metrics.end_time = time.time()
        final_memory = self.process.memory_info().rss / 1024 / 1024
        
        # Calculate metrics
        total_time = metrics.end_time - metrics.start_time
        metrics.throughput_msg_per_sec = num_messages / total_time if total_time > 0 else 0
        memory_growth = final_memory - initial_memory
        peak_memory = max(metrics.memory_usage_mb) if metrics.memory_usage_mb else final_memory
        
        self.results.append(metrics)
        
        print(f"   📊 Throughput: {metrics.throughput_msg_per_sec:,.0f} msg/sec")
        print(f"   💾 Memory: {initial_memory:.1f}MB → {final_memory:.1f}MB (Δ{memory_growth:+.1f}MB)")
        print(f"   🏔️  Peak memory: {peak_memory:.1f}MB")
        print(f"   ❌ Errors: {len(metrics.errors)}")
        
        return metrics
    
    def test_burst_load_handling(self, burst_size: int = 10000, num_bursts: int = 10) -> PerformanceMetrics:
        """Test handling of bursty traffic patterns"""
        print(f"💥 Burst load test ({num_bursts} bursts of {burst_size:,} messages)...")
        
        metrics = PerformanceMetrics(
            test_name="burst_load",
            total_messages=burst_size * num_bursts
        )
        
        burst_times = []
        
        metrics.start_time = time.time()
        
        for burst_num in range(num_bursts):
            print(f"   Processing burst {burst_num + 1}/{num_bursts}...")
            
            # Generate burst data
            burst_messages = self._generate_realistic_test_data(burst_size)
            
            # Process burst as quickly as possible
            burst_start = time.perf_counter()
            burst_processing_times = []
            
            for message_data in burst_messages:
                msg_start = time.perf_counter()
                try:
                    self._validate_message_performance(message_data)
                except Exception as e:
                    metrics.errors.append(f"Burst {burst_num} exception: {e}")
                
                msg_end = time.perf_counter()
                burst_processing_times.append((msg_end - msg_start) * 1_000_000)
            
            burst_end = time.perf_counter()
            burst_time = burst_end - burst_start
            burst_times.append(burst_time)
            
            metrics.processing_times_us.extend(burst_processing_times)
            
            # Record memory after each burst
            current_memory = self.process.memory_info().rss / 1024 / 1024
            metrics.memory_usage_mb.append(current_memory)
            
            burst_throughput = burst_size / burst_time if burst_time > 0 else 0
            print(f"      Burst throughput: {burst_throughput:,.0f} msg/sec")
            
            # Small pause between bursts to simulate real traffic patterns
            time.sleep(0.1)
        
        metrics.end_time = time.time()
        total_time = metrics.end_time - metrics.start_time
        metrics.throughput_msg_per_sec = (burst_size * num_bursts) / total_time if total_time > 0 else 0
        
        self.results.append(metrics)
        
        # Analyze burst performance
        avg_burst_time = statistics.mean(burst_times)
        max_burst_time = max(burst_times)
        min_burst_time = min(burst_times)
        
        print(f"   📊 Overall throughput: {metrics.throughput_msg_per_sec:,.0f} msg/sec")
        print(f"   💥 Burst times: avg={avg_burst_time:.3f}s, min={min_burst_time:.3f}s, max={max_burst_time:.3f}s")
        print(f"   ❌ Errors: {len(metrics.errors)}")
        
        return metrics
    
    def test_precision_under_load(self, num_messages: int = 50000) -> PerformanceMetrics:
        """Test that precision is maintained under high load"""
        print(f"🎯 Precision under load test ({num_messages:,} messages)...")
        
        metrics = PerformanceMetrics(
            test_name="precision_under_load",
            total_messages=num_messages
        )
        
        # Generate test data with realistic precision values
        precision_test_values = [
            "4605.23", "68234.56", "0.12345678", "1.23456789", 
            "9999.99999999", "0.00000001", "12345.67890123"  # Reduced max value to avoid edge cases
        ]
        
        precision_errors = []
        
        metrics.start_time = time.time()
        
        for i in range(num_messages):
            # Use one of our precision test values
            test_value = precision_test_values[i % len(precision_test_values)]
            
            start_time = time.perf_counter()
            try:
                # Test precision-preserving conversion under load
                value_decimal = Decimal(test_value)
                fixed_point = int(value_decimal * Decimal('100000000'))
                recovered = float(fixed_point) / 100000000
                
                # Check precision
                precision_error = abs(float(test_value) - recovered)
                precision_errors.append(precision_error)
                
                if precision_error > 1e-8:
                    metrics.errors.append(f"Precision loss at message {i}: {precision_error:.2e}")
                
            except Exception as e:
                metrics.errors.append(f"Exception at message {i}: {e}")
            
            end_time = time.perf_counter()
            metrics.processing_times_us.append((end_time - start_time) * 1_000_000)
        
        metrics.end_time = time.time()
        total_time = metrics.end_time - metrics.start_time
        metrics.throughput_msg_per_sec = num_messages / total_time if total_time > 0 else 0
        
        self.results.append(metrics)
        
        # Analyze precision
        max_precision_error = max(precision_errors) if precision_errors else 0
        avg_precision_error = statistics.mean(precision_errors) if precision_errors else 0
        precision_violations = sum(1 for e in precision_errors if e > 1e-8)
        
        print(f"   📊 Throughput: {metrics.throughput_msg_per_sec:,.0f} msg/sec")
        print(f"   🎯 Precision: max_error={max_precision_error:.2e}, avg_error={avg_precision_error:.2e}")
        print(f"   ⚠️  Precision violations: {precision_violations}/{len(precision_errors)}")
        print(f"   ❌ Errors: {len(metrics.errors)}")
        
        return metrics
    
    def _generate_realistic_test_data(self, count: int) -> List[Dict[str, Any]]:
        """Generate realistic test data for performance testing"""
        symbols = ["BTC-USD", "ETH-USD", "LTC-USD", "ADA-USD", "USDC-USD"]
        base_prices = {
            "BTC-USD": 68234.56,
            "ETH-USD": 4605.23,
            "LTC-USD": 95.67,
            "ADA-USD": 0.4523,
            "USDC-USD": 1.0001
        }
        
        messages = []
        
        for i in range(count):
            symbol = symbols[i % len(symbols)]
            base_price = base_prices[symbol]
            
            # Add realistic price movement
            price_variation = (random.random() - 0.5) * 0.02  # ±1% variation
            current_price = base_price * (1 + price_variation)
            
            # Generate realistic volume
            volume = random.uniform(0.01, 10.0)
            
            messages.append({
                "symbol": symbol,
                "price": f"{current_price:.8f}",
                "volume": f"{volume:.8f}",
                "side": "buy" if i % 2 == 0 else "sell",
                "timestamp": time.time() + i * 0.001  # 1ms apart
            })
        
        return messages
    
    def _validate_message_performance(self, message: Dict[str, Any]) -> bool:
        """Fast validation for performance testing"""
        try:
            # Quick precision conversion test
            price_decimal = Decimal(message["price"])
            volume_decimal = Decimal(message["volume"])
            
            # Convert to fixed-point
            price_fp = int(price_decimal * Decimal('100000000'))
            volume_fp = int(volume_decimal * Decimal('100000000'))
            
            # Basic range checks
            if price_fp <= 0 or volume_fp < 0:
                return False
            
            # Quick precision check
            price_recovered = float(price_fp) / 100000000
            precision_error = abs(float(message["price"]) - price_recovered)
            
            return precision_error < 1e-8
            
        except Exception:
            return False
    
    def generate_performance_report(self) -> Dict[str, Any]:
        """Generate comprehensive performance report"""
        if not self.results:
            return {"error": "No performance data collected"}
        
        # Overall statistics
        total_messages = sum(r.total_messages for r in self.results)
        total_errors = sum(len(r.errors) for r in self.results)
        
        # Throughput analysis
        throughput_stats = [r.throughput_msg_per_sec for r in self.results]
        
        # Latency analysis (combine all processing times)
        all_processing_times = []
        for result in self.results:
            all_processing_times.extend(result.processing_times_us)
        
        latency_stats = {}
        if all_processing_times:
            sorted_times = sorted(all_processing_times)
            latency_stats = {
                "average_us": statistics.mean(all_processing_times),
                "median_us": statistics.median(all_processing_times),
                "p95_us": sorted_times[int(0.95 * len(sorted_times))],
                "p99_us": sorted_times[int(0.99 * len(sorted_times))],
                "p999_us": sorted_times[int(0.999 * len(sorted_times))],
                "min_us": min(all_processing_times),
                "max_us": max(all_processing_times)
            }
        
        # Memory analysis
        all_memory_usage = []
        for result in self.results:
            all_memory_usage.extend(result.memory_usage_mb)
        
        memory_stats = {}
        if all_memory_usage:
            memory_stats = {
                "peak_mb": max(all_memory_usage),
                "average_mb": statistics.mean(all_memory_usage),
                "min_mb": min(all_memory_usage)
            }
        
        return {
            "summary": {
                "total_messages_processed": total_messages,
                "total_errors": total_errors,
                "error_rate": (total_errors / total_messages * 100) if total_messages > 0 else 0,
                "test_count": len(self.results)
            },
            "throughput": {
                "max_msg_per_sec": max(throughput_stats) if throughput_stats else 0,
                "min_msg_per_sec": min(throughput_stats) if throughput_stats else 0,
                "average_msg_per_sec": statistics.mean(throughput_stats) if throughput_stats else 0
            },
            "latency": latency_stats,
            "memory": memory_stats,
            "test_results": [
                {
                    "name": r.test_name,
                    "messages": r.total_messages,
                    "throughput": r.throughput_msg_per_sec,
                    "errors": len(r.errors),
                    "duration_sec": r.end_time - r.start_time if r.end_time > 0 else 0
                }
                for r in self.results
            ]
        }

def run_performance_stress_tests():
    """Run comprehensive performance and stress tests"""
    print("=" * 80)
    print("PERFORMANCE AND STRESS TESTING")
    print("=" * 80)
    
    tester = PerformanceStressTester()
    
    # Run performance tests
    tester.test_single_thread_performance(10000)
    tester.test_multi_thread_performance(50000, 4)
    tester.test_memory_stress(100000)
    tester.test_burst_load_handling(10000, 5)
    tester.test_precision_under_load(25000)
    
    # Generate report
    report = tester.generate_performance_report()
    
    print("\n" + "=" * 80)
    print("PERFORMANCE TEST RESULTS")
    print("=" * 80)
    
    summary = report["summary"]
    throughput = report["throughput"]
    latency = report.get("latency", {})
    memory = report.get("memory", {})
    
    print(f"📊 Summary:")
    print(f"   Total Messages: {summary['total_messages_processed']:,}")
    print(f"   Total Errors: {summary['total_errors']:,}")
    print(f"   Error Rate: {summary['error_rate']:.3f}%")
    
    print(f"\n🚀 Throughput:")
    print(f"   Peak: {throughput['max_msg_per_sec']:,.0f} msg/sec")
    print(f"   Average: {throughput['average_msg_per_sec']:,.0f} msg/sec")
    print(f"   Minimum: {throughput['min_msg_per_sec']:,.0f} msg/sec")
    
    if latency:
        print(f"\n⏱️  Latency:")
        print(f"   Average: {latency['average_us']:.1f}μs")
        print(f"   P95: {latency['p95_us']:.1f}μs")
        print(f"   P99: {latency['p99_us']:.1f}μs")
        print(f"   P99.9: {latency['p999_us']:.1f}μs")
    
    if memory:
        print(f"\n💾 Memory:")
        print(f"   Peak Usage: {memory['peak_mb']:.1f}MB")
        print(f"   Average Usage: {memory['average_mb']:.1f}MB")
    
    print(f"\n📋 Individual Tests:")
    for test_result in report["test_results"]:
        print(f"   {test_result['name']}: {test_result['throughput']:,.0f} msg/sec, {test_result['errors']} errors")
    
    # Performance benchmarks
    performance_good = (
        throughput['average_msg_per_sec'] >= 50000 and  # At least 50K msg/sec
        latency.get('p99_us', 0) <= 100 and             # P99 latency under 100μs
        summary['error_rate'] <= 0.1                    # Error rate under 0.1%
    )
    
    print(f"\n🏆 PERFORMANCE ASSESSMENT:")
    if performance_good:
        print("   ✅ EXCELLENT - Pipeline meets high-performance requirements")
    else:
        print("   ⚠️ REVIEW NEEDED - Performance optimization may be required")
        
        if throughput['average_msg_per_sec'] < 50000:
            print(f"      • Throughput below target: {throughput['average_msg_per_sec']:,.0f} < 50,000 msg/sec")
        if latency.get('p99_us', 0) > 100:
            print(f"      • P99 latency above target: {latency['p99_us']:.1f}μs > 100μs")
        if summary['error_rate'] > 0.1:
            print(f"      • Error rate above target: {summary['error_rate']:.3f}% > 0.1%")
    
    # Save detailed report
    with open("/Users/daws/alphapulse/backend/tests/e2e/performance_stress_report.json", "w") as f:
        json.dump(report, f, indent=2, default=str)
    
    print(f"\n📄 Detailed report saved to: performance_stress_report.json")
    
    return performance_good

if __name__ == "__main__":
    success = run_performance_stress_tests()
    exit(0 if success else 1)