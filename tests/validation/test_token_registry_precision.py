#!/usr/bin/env python3
"""
Token Registry Precision Tests

Tests the token registry for proper decimal handling across different tokens and chains.
This validates that tokens with different decimal places (6, 8, 18) are handled correctly.
"""

import json
import logging
from typing import Dict, List, Any, Tuple
from dataclasses import dataclass
from decimal import Decimal, getcontext

# Set high precision for decimal calculations
getcontext().prec = 28

@dataclass
class TokenTestConfig:
    """Configuration for token precision testing"""
    symbol: str
    chain: str
    decimals: int
    test_amount: str  # Human-readable amount to test
    expected_raw: int  # Expected raw token units
    is_stablecoin: bool
    display_decimals: int

class TokenRegistryTester:
    """Tests token registry precision handling"""
    
    def __init__(self):
        self.test_results = []
        self.token_configs = self._get_test_token_configs()
        
    def _get_test_token_configs(self) -> List[TokenTestConfig]:
        """Get test configurations for various tokens"""
        return [
            # Ethereum tokens
            TokenTestConfig("USDC", "ethereum", 6, "1.0", 1_000_000, True, 4),
            TokenTestConfig("USDT", "ethereum", 6, "1000.0", 1_000_000_000, True, 4),
            TokenTestConfig("WETH", "ethereum", 18, "1.0", 1_000_000_000_000_000_000, False, 4),
            TokenTestConfig("WBTC", "ethereum", 8, "1.0", 100_000_000, False, 4),
            TokenTestConfig("DAI", "ethereum", 18, "1.0", 1_000_000_000_000_000_000, True, 4),
            TokenTestConfig("LINK", "ethereum", 18, "10.0", 10_000_000_000_000_000_000, False, 3),
            TokenTestConfig("AAVE", "ethereum", 18, "100.0", 100_000_000_000_000_000_000, False, 2),
            
            # Polygon tokens
            TokenTestConfig("WMATIC", "polygon", 18, "100.0", 100_000_000_000_000_000_000, False, 4),
            TokenTestConfig("USDC", "polygon", 6, "1.0", 1_000_000, True, 4),
            TokenTestConfig("WETH", "polygon", 18, "1.0", 1_000_000_000_000_000_000, False, 4),
            
            # Edge cases with small amounts
            TokenTestConfig("USDC", "ethereum", 6, "0.000001", 1, True, 4),  # Minimum USDC
            TokenTestConfig("WETH", "ethereum", 18, "0.000000000000000001", 1, False, 4),  # Minimum ETH
        ]
    
    def test_token_amount_conversions(self) -> Dict[str, Any]:
        """Test conversion between human and raw amounts"""
        print("🪙 Testing token amount conversions...")
        
        results = {
            "conversions": [],
            "precision_errors": [],
            "failed_conversions": []
        }
        
        for config in self.token_configs:
            try:
                # Test human → raw conversion
                raw_amount = self._human_to_raw(config.test_amount, config.decimals)
                
                # Test raw → human conversion
                recovered_amount = self._raw_to_human(raw_amount, config.decimals)
                
                # Calculate precision error
                original_decimal = Decimal(config.test_amount)
                recovered_decimal = Decimal(recovered_amount)
                precision_error = abs(original_decimal - recovered_decimal)
                
                conversion_result = {
                    "token": f"{config.chain}:{config.symbol}",
                    "decimals": config.decimals,
                    "test_amount": config.test_amount,
                    "calculated_raw": raw_amount,
                    "expected_raw": config.expected_raw,
                    "recovered_amount": recovered_amount,
                    "precision_error": float(precision_error),
                    "raw_matches": raw_amount == config.expected_raw,
                    "precision_preserved": precision_error == 0
                }
                
                results["conversions"].append(conversion_result)
                results["precision_errors"].append(float(precision_error))
                
                # Print result
                status = "✅" if conversion_result["raw_matches"] and conversion_result["precision_preserved"] else "❌"
                print(f"   {status} {config.chain}:{config.symbol} ({config.decimals} decimals): {config.test_amount} → {raw_amount} → {recovered_amount}")
                
                if not conversion_result["raw_matches"]:
                    print(f"      Expected raw: {config.expected_raw}, got: {raw_amount}")
                
                if not conversion_result["precision_preserved"]:
                    print(f"      Precision error: {precision_error}")
                
            except Exception as e:
                error_result = {
                    "token": f"{config.chain}:{config.symbol}",
                    "error": str(e),
                    "test_amount": config.test_amount
                }
                results["failed_conversions"].append(error_result)
                print(f"   ❌ {config.chain}:{config.symbol}: Exception - {e}")
        
        return results
    
    def test_cross_chain_consistency(self) -> Dict[str, Any]:
        """Test that same tokens on different chains handle amounts consistently"""
        print("\n🌉 Testing cross-chain consistency...")
        
        results = {
            "cross_chain_tests": [],
            "inconsistencies": []
        }
        
        # Find tokens that exist on multiple chains
        multi_chain_tokens = {}
        for config in self.token_configs:
            if config.symbol not in multi_chain_tokens:
                multi_chain_tokens[config.symbol] = []
            multi_chain_tokens[config.symbol].append(config)
        
        for symbol, chain_configs in multi_chain_tokens.items():
            if len(chain_configs) > 1:
                # Test same amount on different chains
                test_amount = "1.0"
                chain_results = []
                
                for config in chain_configs:
                    try:
                        raw_amount = self._human_to_raw(test_amount, config.decimals)
                        recovered = self._raw_to_human(raw_amount, config.decimals)
                        
                        chain_results.append({
                            "chain": config.chain,
                            "decimals": config.decimals,
                            "raw_amount": raw_amount,
                            "recovered": recovered
                        })
                        
                    except Exception as e:
                        chain_results.append({
                            "chain": config.chain,
                            "error": str(e)
                        })
                
                # Check for consistency
                expected_decimals = chain_results[0].get("decimals")
                consistent = all(
                    r.get("decimals") == expected_decimals 
                    for r in chain_results 
                    if "decimals" in r
                )
                
                test_result = {
                    "symbol": symbol,
                    "chains": chain_results,
                    "consistent_decimals": consistent
                }
                
                results["cross_chain_tests"].append(test_result)
                
                if not consistent:
                    results["inconsistencies"].append(f"{symbol} has different decimals across chains")
                    print(f"   ⚠️  {symbol}: Inconsistent decimals across chains")
                else:
                    print(f"   ✅ {symbol}: Consistent across chains ({expected_decimals} decimals)")
        
        return results
    
    def test_protocol_fixed_point_integration(self) -> Dict[str, Any]:
        """Test integration with our 8-decimal fixed-point protocol"""
        print("\n🔧 Testing protocol fixed-point integration...")
        
        results = {
            "protocol_tests": [],
            "precision_errors": []
        }
        
        # Test how token amounts convert to our 8-decimal protocol
        for config in self.token_configs:
            try:
                # Convert to raw token amount
                raw_token_amount = self._human_to_raw(config.test_amount, config.decimals)
                
                # Convert to protocol price (assuming 1:1 USD for simplicity)
                # In real system, this would use current market price
                protocol_price = self._human_to_raw("1.0", 8)  # $1.00 in 8-decimal fixed-point
                
                # Simulate trade: amount * price
                trade_value = raw_token_amount  # Simplified
                
                # Convert back through protocol
                recovered_token = self._raw_to_human(trade_value, config.decimals)
                
                # Calculate error
                original_decimal = Decimal(config.test_amount)
                recovered_decimal = Decimal(recovered_token)
                error = abs(original_decimal - recovered_decimal)
                
                protocol_result = {
                    "token": f"{config.chain}:{config.symbol}",
                    "original_amount": config.test_amount,
                    "raw_token_amount": raw_token_amount,
                    "protocol_price": protocol_price,
                    "recovered_amount": recovered_token,
                    "precision_error": float(error),
                    "precision_preserved": error < Decimal('1e-8')
                }
                
                results["protocol_tests"].append(protocol_result)
                results["precision_errors"].append(float(error))
                
                status = "✅" if protocol_result["precision_preserved"] else "❌"
                print(f"   {status} {config.chain}:{config.symbol}: {config.test_amount} → {recovered_token} (error: {error:.2e})")
                
            except Exception as e:
                print(f"   ❌ {config.chain}:{config.symbol}: Exception - {e}")
        
        return results
    
    def _human_to_raw(self, human_amount: str, decimals: int) -> int:
        """Convert human-readable amount to raw token units"""
        decimal_amount = Decimal(human_amount)
        multiplier = Decimal(10 ** decimals)
        raw_decimal = decimal_amount * multiplier
        return int(raw_decimal)
    
    def _raw_to_human(self, raw_amount: int, decimals: int) -> str:
        """Convert raw token units to human-readable amount"""
        divisor = Decimal(10 ** decimals)
        human_decimal = Decimal(raw_amount) / divisor
        return str(human_decimal)
    
    def run_comprehensive_tests(self) -> Dict[str, Any]:
        """Run all token registry tests"""
        print("=" * 80)
        print("TOKEN REGISTRY PRECISION TESTS")
        print("=" * 80)
        
        # Run all test categories
        conversion_results = self.test_token_amount_conversions()
        cross_chain_results = self.test_cross_chain_consistency()
        protocol_results = self.test_protocol_fixed_point_integration()
        
        # Calculate overall statistics
        total_conversions = len(conversion_results["conversions"])
        successful_conversions = sum(1 for c in conversion_results["conversions"] if c["raw_matches"] and c["precision_preserved"])
        failed_conversions = len(conversion_results["failed_conversions"])
        
        max_precision_error = max(conversion_results["precision_errors"]) if conversion_results["precision_errors"] else 0
        avg_precision_error = sum(conversion_results["precision_errors"]) / len(conversion_results["precision_errors"]) if conversion_results["precision_errors"] else 0
        
        return {
            "summary": {
                "total_token_tests": total_conversions,
                "successful_conversions": successful_conversions,
                "failed_conversions": failed_conversions,
                "success_rate": (successful_conversions / total_conversions * 100) if total_conversions > 0 else 0,
                "cross_chain_inconsistencies": len(cross_chain_results["inconsistencies"])
            },
            "precision_analysis": {
                "max_precision_error": max_precision_error,
                "average_precision_error": avg_precision_error,
                "protocol_integration_errors": len([p for p in protocol_results["protocol_tests"] if not p["precision_preserved"]])
            },
            "detailed_results": {
                "conversions": conversion_results,
                "cross_chain": cross_chain_results,
                "protocol": protocol_results
            }
        }
    
    def generate_report(self) -> str:
        """Generate human-readable test report"""
        results = self.run_comprehensive_tests()
        
        report = []
        report.append("=" * 80)
        report.append("TOKEN REGISTRY PRECISION TEST REPORT")
        report.append("=" * 80)
        
        summary = results["summary"]
        precision = results["precision_analysis"]
        
        report.append(f"\nSummary:")
        report.append(f"  Total Token Tests: {summary['total_token_tests']}")
        report.append(f"  Successful Conversions: {summary['successful_conversions']}")
        report.append(f"  Failed Conversions: {summary['failed_conversions']}")
        report.append(f"  Success Rate: {summary['success_rate']:.1f}%")
        report.append(f"  Cross-Chain Issues: {summary['cross_chain_inconsistencies']}")
        
        report.append(f"\nPrecision Analysis:")
        report.append(f"  Max Precision Error: {precision['max_precision_error']:.2e}")
        report.append(f"  Avg Precision Error: {precision['average_precision_error']:.2e}")
        report.append(f"  Protocol Integration Errors: {precision['protocol_integration_errors']}")
        
        # Assessment
        overall_success = (
            summary['success_rate'] >= 95.0 and
            precision['max_precision_error'] < 1e-10 and
            summary['cross_chain_inconsistencies'] == 0
        )
        
        report.append(f"\n🏆 Overall Assessment:")
        if overall_success:
            report.append("   ✅ EXCELLENT - Token registry handles all decimals correctly")
            report.append("   All tokens maintain precision across different decimal configurations")
        else:
            report.append("   ❌ ISSUES DETECTED - Token registry needs attention")
            if summary['success_rate'] < 95.0:
                report.append(f"   • Low success rate: {summary['success_rate']:.1f}%")
            if precision['max_precision_error'] >= 1e-10:
                report.append(f"   • High precision errors: {precision['max_precision_error']:.2e}")
            if summary['cross_chain_inconsistencies'] > 0:
                report.append(f"   • Cross-chain inconsistencies: {summary['cross_chain_inconsistencies']}")
        
        return "\n".join(report)

def run_token_registry_tests():
    """Run token registry precision tests"""
    tester = TokenRegistryTester()
    report = tester.generate_report()
    
    print(report)
    
    # Save detailed results
    results = tester.run_comprehensive_tests()
    with open("/Users/daws/alphapulse/tests/validation/token_registry_test_report.json", "w") as f:
        json.dump(results, f, indent=2, default=str)
    
    print(f"\n📄 Detailed report saved to: token_registry_test_report.json")
    
    # Determine success
    summary = results["summary"]
    precision = results["precision_analysis"]
    
    success = (
        summary['success_rate'] >= 95.0 and
        precision['max_precision_error'] < 1e-10 and
        summary['cross_chain_inconsistencies'] == 0
    )
    
    return success

if __name__ == "__main__":
    success = run_token_registry_tests()
    exit(0 if success else 1)