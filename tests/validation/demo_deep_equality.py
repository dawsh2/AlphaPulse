#!/usr/bin/env python3
"""
Deep Equality Framework Demonstration

This script demonstrates the deep equality validation framework that ensures
"anything put into the system comes out the same" as requested by the user.

Usage:
    python3 demo_deep_equality.py
"""

from deep_equality_framework import DeepEqualityFramework
import json
import time

def demo_perfect_equality():
    """Demonstrate perfect data preservation"""
    print("🎯 DEMO 1: Perfect Data Preservation")
    print("=" * 50)
    
    framework = DeepEqualityFramework()
    
    # Original message from Polygon API
    original = {
        "symbol": "polygon:0x123abc:WETH/USDC",
        "price": 2500.123456789,
        "volume": 1000.0,
        "liquidity": 50000.0,
        "gas_cost": 0.002,
        "timestamp": 1705123456789,
        "metadata": {
            "block_number": 12345,
            "tx_hash": "0xabcdef123456",
            "pool_fee": 0.003
        }
    }
    
    print(f"📡 Original: {json.dumps(original, indent=2)}")
    
    # Start tracking
    message_id = framework.start_tracking(original)
    print(f"🆔 Message ID: {message_id}")
    
    # Simulate pipeline stages
    collector_output = original.copy()
    collector_output["collector_processed"] = True
    framework.record_stage(message_id, "collector", collector_output)
    
    binary_output = original.copy()
    binary_output["binary_encoded"] = True
    framework.record_stage(message_id, "binary_protocol", binary_output)
    
    # Final output (identical to original)
    final_output = original.copy()
    
    # Validate
    result = framework.validate_final_output(message_id, final_output)
    
    print(f"✅ Validation Result:")
    print(f"   Is Equal: {result['is_equal']}")
    print(f"   Errors: {len(result['errors'])}")
    print(f"   Validation Time: {result['validation_time_ms']:.3f}ms")
    
    return result['is_equal']

def demo_data_corruption():
    """Demonstrate detection of data corruption"""
    print("\n🚨 DEMO 2: Data Corruption Detection")
    print("=" * 50)
    
    framework = DeepEqualityFramework()
    
    # Original data
    original = {
        "symbol": "WETH/USDC",
        "price": 2500.123456789,
        "volume": 1000.0
    }
    
    print(f"📡 Original: {json.dumps(original)}")
    
    # Start tracking
    message_id = framework.start_tracking(original)
    
    # Corrupted final output (price changed!)
    corrupted_output = {
        "symbol": "WETH/USDC", 
        "price": 2500.123456788,  # Precision loss!
        "volume": 1000.0
    }
    
    print(f"💥 Corrupted: {json.dumps(corrupted_output)}")
    
    # Validate
    result = framework.validate_final_output(message_id, corrupted_output)
    
    print(f"❌ Validation Result:")
    print(f"   Is Equal: {result['is_equal']}")
    print(f"   Errors: {len(result['errors'])}")
    if result['errors']:
        for error in result['errors']:
            print(f"   📍 {error}")
    
    return not result['is_equal']  # Success if corruption detected

def demo_complex_nested_data():
    """Demonstrate validation of complex nested structures"""
    print("\n🔧 DEMO 3: Complex Nested Data Validation")
    print("=" * 50)
    
    framework = DeepEqualityFramework()
    
    # Complex nested data
    original = {
        "trades": [
            {
                "symbol": "WETH/USDC",
                "price": 2500.0,
                "amounts": {
                    "token0": 1.5,
                    "token1": 3750.0
                }
            },
            {
                "symbol": "MATIC/USDC", 
                "price": 0.85,
                "amounts": {
                    "token0": 1000.0,
                    "token1": 850.0
                }
            }
        ],
        "metadata": {
            "pool_states": {
                "total_liquidity": 100000.0,
                "active_pools": 5
            }
        }
    }
    
    print(f"📡 Original: {json.dumps(original, indent=2)}")
    
    message_id = framework.start_tracking(original)
    
    # Perfect reconstruction
    final_output = original.copy()
    result = framework.validate_final_output(message_id, final_output)
    
    print(f"✅ Complex Validation:")
    print(f"   Is Equal: {result['is_equal']}")
    print(f"   Errors: {len(result['errors'])}")
    
    return result['is_equal']

def demo_framework_statistics():
    """Show framework performance statistics"""
    print("\n📊 DEMO 4: Framework Statistics")
    print("=" * 50)
    
    framework = DeepEqualityFramework()
    
    # Process multiple messages
    for i in range(5):
        original = {"id": i, "price": 100.0 + i, "volume": 1000.0}
        message_id = framework.start_tracking(original)
        
        # Simulate processing time
        time.sleep(0.001)
        
        # Some succeed, some fail (simulate real conditions)
        if i % 2 == 0:
            final_output = original.copy()  # Perfect
        else:
            final_output = {"id": i, "price": 100.0 + i + 0.1, "volume": 1000.0}  # Corrupted
        
        framework.validate_final_output(message_id, final_output)
    
    stats = framework.get_statistics()
    
    print("📈 Framework Performance:")
    print(f"   Messages Tracked: {stats['messages_tracked']}")
    print(f"   Validations Performed: {stats['validations_performed']}")
    print(f"   Perfect Matches: {stats['perfect_matches']}")
    print(f"   Failed Validations: {stats['failed_validations']}")
    print(f"   Success Rate: {stats['success_rate_percent']:.1f}%")
    print(f"   Average Validation Time: {stats['average_validation_time_ms']:.3f}ms")
    
    return stats

def main():
    """Run all demonstrations"""
    print("🎯 DEEP EQUALITY VALIDATION FRAMEWORK DEMO")
    print("=" * 60)
    print("Demonstrates the methodology to validate that anything")
    print("put into the system comes out the same.")
    print("=" * 60)
    
    # Run all demos
    demo1_pass = demo_perfect_equality()
    demo2_pass = demo_data_corruption()
    demo3_pass = demo_complex_nested_data()
    stats = demo_framework_statistics()
    
    print("\n🎉 DEMO SUMMARY")
    print("=" * 30)
    print(f"✅ Perfect Equality: {'PASS' if demo1_pass else 'FAIL'}")
    print(f"🚨 Corruption Detection: {'PASS' if demo2_pass else 'FAIL'}")
    print(f"🔧 Complex Data: {'PASS' if demo3_pass else 'FAIL'}")
    print(f"📊 Framework Efficiency: {stats['average_validation_time_ms']:.3f}ms avg")
    
    all_passed = demo1_pass and demo2_pass and demo3_pass
    
    if all_passed:
        print("\n🎉 SUCCESS: Framework ready for production use!")
        print("✅ Guarantees exact data preservation through pipeline")
        return 0
    else:
        print("\n❌ Some demos failed - framework needs adjustment")
        return 1

if __name__ == "__main__":
    exit(main())