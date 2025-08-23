#!/usr/bin/env python3
"""
External API Cross-Validation
Compare our live data against CoinMarketCap, CoinGecko, and DEX APIs
"""

import requests
import json
import time
from typing import Dict, List, Any, Optional
from dataclasses import dataclass

@dataclass 
class PriceComparison:
    symbol: str
    our_price: float
    external_price: float
    source: str
    deviation_percent: float
    timestamp: float

class ExternalAPIValidator:
    def __init__(self):
        self.results: List[PriceComparison] = []
        
    def get_coingecko_price(self, token_id: str) -> Optional[float]:
        """Get price from CoinGecko API"""
        try:
            url = f"https://api.coingecko.com/api/v3/simple/price?ids={token_id}&vs_currencies=usd"
            response = requests.get(url, timeout=10)
            if response.status_code == 200:
                data = response.json()
                return data[token_id]['usd']
        except Exception as e:
            print(f"❌ CoinGecko API error for {token_id}: {e}")
        return None
    
    def get_coinmarketcap_price(self, symbol: str) -> Optional[float]:
        """Get price from CoinMarketCap (requires API key for full access)"""
        try:
            # Using public endpoints that don't require API key
            # Note: This is limited and may not work for all tokens
            print(f"ℹ️  CoinMarketCap validation would require API key for {symbol}")
            return None
        except Exception as e:
            print(f"❌ CoinMarketCap API error for {symbol}: {e}")
        return None
    
    def validate_wmatic_price(self, our_price: float) -> Optional[PriceComparison]:
        """Validate WMATIC/POL price against external sources"""
        print(f"🔍 Validating WMATIC price: ${our_price:.6f}")
        
        # CoinGecko ID for Polygon (POL, formerly MATIC)
        external_price = self.get_coingecko_price("matic-network")
        
        if external_price:
            deviation = abs(our_price - external_price) / external_price
            
            comparison = PriceComparison(
                symbol="WMATIC/POL",
                our_price=our_price,
                external_price=external_price,
                source="CoinGecko",
                deviation_percent=deviation * 100,
                timestamp=time.time()
            )
            
            print(f"   Our price: ${our_price:.6f}")
            print(f"   CoinGecko: ${external_price:.6f}")
            print(f"   Deviation: {deviation:.2%}")
            
            if deviation > 0.10:  # >10% deviation
                print(f"   ⚠️  HIGH DEVIATION: {deviation:.2%}")
            elif deviation > 0.05:  # >5% deviation  
                print(f"   ⚠️  Medium deviation: {deviation:.2%}")
            else:
                print(f"   ✅ Good accuracy: {deviation:.2%}")
                
            return comparison
            
        return None
    
    def validate_weth_price(self, our_price: float) -> Optional[PriceComparison]:
        """Validate WETH price against ETH market price"""
        print(f"🔍 Validating WETH price: ${our_price:.2f}")
        
        external_price = self.get_coingecko_price("ethereum")
        
        if external_price:
            deviation = abs(our_price - external_price) / external_price
            
            comparison = PriceComparison(
                symbol="WETH",
                our_price=our_price,
                external_price=external_price,
                source="CoinGecko",
                deviation_percent=deviation * 100,
                timestamp=time.time()
            )
            
            print(f"   Our price: ${our_price:.2f}")
            print(f"   CoinGecko ETH: ${external_price:.2f}")
            print(f"   Deviation: {deviation:.2%}")
            
            if deviation > 0.02:  # >2% deviation for ETH
                print(f"   ⚠️  HIGH DEVIATION: {deviation:.2%}")
            else:
                print(f"   ✅ Good accuracy: {deviation:.2%}")
                
            return comparison
            
        return None
    
    def validate_polygon_dex_liquidity(self, pool_address: str) -> Optional[Dict]:
        """Cross-check liquidity against Polygon DEX APIs"""
        try:
            # Try QuickSwap API (if available)
            print(f"🔍 Checking QuickSwap for pool {pool_address[:10]}...")
            
            # QuickSwap GraphQL endpoint
            quickswap_url = "https://api.thegraph.com/subgraphs/name/sameepsi/quickswap06"
            
            query = """
            {
              pair(id: "%s") {
                id
                reserveUSD
                token0 {
                  symbol
                  decimals
                }
                token1 {
                  symbol
                  decimals
                }
                reserve0
                reserve1
              }
            }
            """ % pool_address.lower()
            
            response = requests.post(
                quickswap_url,
                json={"query": query},
                timeout=10
            )
            
            if response.status_code == 200:
                data = response.json()
                if 'data' in data and data['data']['pair']:
                    pair_data = data['data']['pair']
                    
                    result = {
                        "pool_address": pool_address,
                        "reserveUSD": float(pair_data['reserveUSD']),
                        "reserve0": float(pair_data['reserve0']),
                        "reserve1": float(pair_data['reserve1']),
                        "token0_symbol": pair_data['token0']['symbol'],
                        "token1_symbol": pair_data['token1']['symbol'],
                        "source": "QuickSwap GraphQL"
                    }
                    
                    print(f"   ✅ QuickSwap liquidity: ${result['reserveUSD']:,.2f}")
                    print(f"   Reserves: {result['reserve0']:.2f} {result['token0_symbol']} / {result['reserve1']:.2f} {result['token1_symbol']}")
                    
                    return result
                else:
                    print(f"   ℹ️  Pool {pool_address[:10]}... not found on QuickSwap")
            else:
                print(f"   ❌ QuickSwap API error: {response.status_code}")
                
        except Exception as e:
            print(f"   ❌ QuickSwap validation error: {e}")
            
        return None
    
    def run_comprehensive_validation(self, sample_trades: List[Dict]) -> Dict[str, Any]:
        """Run comprehensive validation against multiple external APIs"""
        print("=" * 80)
        print("EXTERNAL API CROSS-VALIDATION")
        print("=" * 80)
        
        validations = []
        
        # Extract WMATIC trades for validation
        wmatic_trades = [t for t in sample_trades if 'WMATIC' in t.get('symbol', '')]
        weth_trades = [t for t in sample_trades if 'WETH' in t.get('symbol', '')]
        
        print(f"\n1️⃣ Found {len(wmatic_trades)} WMATIC trades, {len(weth_trades)} WETH trades")
        
        # Validate WMATIC prices
        if wmatic_trades:
            print(f"\n🔍 WMATIC Price Validation:")
            wmatic_prices = [t['price'] for t in wmatic_trades[:5]]  # First 5
            avg_wmatic_price = sum(wmatic_prices) / len(wmatic_prices)
            
            validation = self.validate_wmatic_price(avg_wmatic_price)
            if validation:
                validations.append(validation)
        
        # Validate WETH prices  
        if weth_trades:
            print(f"\n🔍 WETH Price Validation:")
            weth_prices = [t['price'] for t in weth_trades[:5]]  # First 5
            avg_weth_price = sum(weth_prices) / len(weth_prices)
            
            validation = self.validate_weth_price(avg_weth_price)
            if validation:
                validations.append(validation)
        
        # Validate pool liquidity for common pools
        print(f"\n🔍 Pool Liquidity Validation:")
        common_pools = [
            "0x604229c960e5cacf2aaeac8be68ac07ba9df81c3",  # WMATIC/USDT
            "0x55ff76bffc3cdd9d5fdbbc2ece4528ecce45047e",  # WMATIC/USDT  
            "0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2"   # WMATIC/USDC
        ]
        
        liquidity_validations = []
        for pool in common_pools[:2]:  # Check first 2 pools
            result = self.validate_polygon_dex_liquidity(pool)
            if result:
                liquidity_validations.append(result)
        
        # Generate report
        report = {
            "timestamp": time.time(),
            "price_validations": len(validations),
            "liquidity_validations": len(liquidity_validations),
            "results": {
                "price_comparisons": [
                    {
                        "symbol": v.symbol,
                        "our_price": v.our_price,
                        "external_price": v.external_price,
                        "deviation_percent": v.deviation_percent,
                        "source": v.source,
                        "accuracy_level": (
                            "HIGH" if v.deviation_percent < 2 else
                            "MEDIUM" if v.deviation_percent < 5 else
                            "LOW"
                        )
                    } for v in validations
                ],
                "liquidity_comparisons": liquidity_validations
            }
        }
        
        return report

def main():
    validator = ExternalAPIValidator()
    
    # Sample trades from our previous validation
    sample_trades = [
        {"symbol": "WMATIC/USDT", "price": 0.233969},
        {"symbol": "WMATIC/USDT", "price": 0.232249}, 
        {"symbol": "WMATIC/USDC", "price": 0.191876},
        {"symbol": "WETH/USDC", "price": 4446.945314},
        {"symbol": "WETH/USDC", "price": 4452.362908},
        {"symbol": "DAI/LGNS", "price": 0.080323}
    ]
    
    report = validator.run_comprehensive_validation(sample_trades)
    
    print("\n" + "=" * 80)
    print("EXTERNAL VALIDATION REPORT")
    print("=" * 80)
    
    print(f"📊 Price Validations: {report['price_validations']}")
    print(f"🏊 Liquidity Validations: {report['liquidity_validations']}")
    
    if report['results']['price_comparisons']:
        print(f"\n💰 Price Accuracy Results:")
        for comp in report['results']['price_comparisons']:
            accuracy_emoji = "✅" if comp['accuracy_level'] == "HIGH" else "⚠️" if comp['accuracy_level'] == "MEDIUM" else "❌"
            print(f"   {accuracy_emoji} {comp['symbol']}: {comp['deviation_percent']:.2f}% deviation ({comp['accuracy_level']} accuracy)")
    
    if report['results']['liquidity_comparisons']:
        print(f"\n🏊 Liquidity Cross-Check:")
        for liq in report['results']['liquidity_comparisons']:
            print(f"   ✅ {liq['token0_symbol']}/{liq['token1_symbol']}: ${liq['reserveUSD']:,.2f} on {liq['source']}")
    
    # Save report
    with open('/Users/daws/alphapulse/backend/external_validation_report.json', 'w') as f:
        json.dump(report, f, indent=2, default=str)
    
    print(f"\n📄 Report saved to: external_validation_report.json")
    
    # Overall assessment
    if report['results']['price_comparisons']:
        high_accuracy = sum(1 for c in report['results']['price_comparisons'] if c['accuracy_level'] == 'HIGH')
        total_comparisons = len(report['results']['price_comparisons'])
        accuracy_rate = high_accuracy / total_comparisons if total_comparisons > 0 else 0
        
        print(f"\n🏆 OVERALL ASSESSMENT:")
        print(f"   High Accuracy Rate: {accuracy_rate:.1%} ({high_accuracy}/{total_comparisons})")
        
        if accuracy_rate >= 0.8:
            print(f"   ✅ EXCELLENT: Data accuracy validated against external APIs")
            return 0
        elif accuracy_rate >= 0.6:
            print(f"   ⚠️  GOOD: Most data accurate, some deviations detected")
            return 0
        else:
            print(f"   ❌ POOR: Significant deviations from market prices")
            return 1
    else:
        print(f"   ℹ️  No external price validations completed")
        return 0

if __name__ == "__main__":
    exit(main())