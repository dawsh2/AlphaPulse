#!/usr/bin/env python3
"""
Real Test Suite Runner

Runs both real E2E tests to validate the pipeline uses real components instead of mocks.
"""

import asyncio
import sys
import os
from pathlib import Path

# Add the backend directory to path so we can import the tests
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from tests.e2e.test_real_data_pipeline import main as run_real_pipeline_test

async def run_all_real_tests():
    """Run all real E2E tests"""
    print("🧪 ALPHAPULSE REAL E2E TEST SUITE")
    print("=" * 60)
    print("Testing actual components with real data flow")
    print("NO SIMULATION - validates actual pipeline")
    print("=" * 60)
    
    test_results = []
    
    # Test 1: Real Data Pipeline Test
    print("\n📊 TEST 1: Real Data Pipeline")
    print("-" * 40)
    try:
        # Run the real pipeline test
        result = await run_real_pipeline_test()
        test_results.append(("Real Data Pipeline", result))
        if result:
            print("✅ Real Data Pipeline Test: PASSED")
        else:
            print("❌ Real Data Pipeline Test: FAILED")
    except Exception as e:
        print(f"❌ Real Data Pipeline Test: ERROR - {e}")
        test_results.append(("Real Data Pipeline", False))
    
    # Test Summary
    print("\n" + "=" * 60)
    print("TEST SUITE SUMMARY")
    print("=" * 60)
    
    passed_tests = sum(1 for _, result in test_results if result)
    total_tests = len(test_results)
    
    for test_name, result in test_results:
        status = "✅ PASSED" if result else "❌ FAILED"
        print(f"   {test_name}: {status}")
    
    print(f"\nOverall: {passed_tests}/{total_tests} tests passed")
    
    if passed_tests == total_tests:
        print("\n🏆 ALL TESTS PASSED!")
        print("   The pipeline is using REAL components")
        print("   SymbolMapping and Trade messages flow correctly")
        print("   Dashboard displays human-readable symbols")
        return True
    else:
        print("\n💥 SOME TESTS FAILED")
        print("   Check component status and connections")
        return False

def main():
    """Main entry point"""
    try:
        success = asyncio.run(run_all_real_tests())
        sys.exit(0 if success else 1)
    except KeyboardInterrupt:
        print("\n⚠️  Tests interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\n💥 Test suite failed: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()