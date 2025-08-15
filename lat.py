#!/usr/bin/env python3
"""
RPC Endpoint Latency Tester
Tests latency across free RPC endpoints for various blockchains
"""

import requests
import time
import json
import statistics
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Dict, List, Tuple
import argparse

# Free RPC endpoints to test
ENDPOINTS = {
    'ethereum': [
        {'name': 'Ankr', 'url': 'https://rpc.ankr.com/eth'},
        {'name': 'Public RPC', 'url': 'https://eth.public-rpc.com'},
        {'name': 'PublicNode', 'url': 'https://ethereum.publicnode.com'},
        {'name': 'LlamaRPC', 'url': 'https://eth.llamarpc.com'},
        {'name': 'Bloxroute', 'url': 'https://virginia.rpc.blxrbdn.com'},
        {'name': 'CloudFlare', 'url': 'https://cloudflare-eth.com'},
    ],
    'polygon': [
        {'name': 'Ankr', 'url': 'https://rpc.ankr.com/polygon'},
        {'name': 'Polygon Official', 'url': 'https://polygon-rpc.com'},
        {'name': 'PublicNode', 'url': 'https://polygon.publicnode.com'},
        {'name': 'LlamaRPC', 'url': 'https://polygon.llamarpc.com'},
        {'name': 'Matic Network', 'url': 'https://rpc-mainnet.matic.network'},
    ],
    'bsc': [
        {'name': 'Ankr', 'url': 'https://rpc.ankr.com/bsc'},
        {'name': 'Binance Official', 'url': 'https://bsc-dataseed.binance.org'},
        {'name': 'PublicNode', 'url': 'https://bsc.publicnode.com'},
        {'name': 'LlamaRPC', 'url': 'https://bsc.llamarpc.com'},
        {'name': 'DeFiBit', 'url': 'https://bsc-dataseed1.defibit.io'},
    ],
    'arbitrum': [
        {'name': 'Ankr', 'url': 'https://rpc.ankr.com/arbitrum'},
        {'name': 'Arbitrum Official', 'url': 'https://arb1.arbitrum.io/rpc'},
        {'name': 'PublicNode', 'url': 'https://arbitrum.publicnode.com'},
        {'name': 'LlamaRPC', 'url': 'https://arbitrum.llamarpc.com'},
    ],
    'optimism': [
        {'name': 'Ankr', 'url': 'https://rpc.ankr.com/optimism'},
        {'name': 'Optimism Official', 'url': 'https://mainnet.optimism.io'},
        {'name': 'PublicNode', 'url': 'https://optimism.publicnode.com'},
    ],
    'avalanche': [
        {'name': 'Ankr', 'url': 'https://rpc.ankr.com/avalanche'},
        {'name': 'Avalanche Official', 'url': 'https://api.avax.network/ext/bc/C/rpc'},
        {'name': 'PublicNode', 'url': 'https://avalanche.publicnode.com'},
    ]
}

def test_endpoint(endpoint: Dict, chain: str, num_tests: int = 5) -> Dict:
    """Test a single RPC endpoint"""
    payload = {
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    }
    
    headers = {
        "Content-Type": "application/json"
    }
    
    latencies = []
    errors = []
    block_number = None
    
    for i in range(num_tests):
        try:
            start_time = time.time()
            response = requests.post(
                endpoint['url'], 
                json=payload, 
                headers=headers, 
                timeout=10
            )
            end_time = time.time()
            
            latency = (end_time - start_time) * 1000  # Convert to milliseconds
            
            if response.status_code == 200:
                data = response.json()
                if 'result' in data:
                    latencies.append(latency)
                    if block_number is None:
                        block_number = int(data['result'], 16)
                else:
                    errors.append(f"No result in response: {data}")
            else:
                errors.append(f"HTTP {response.status_code}")
                
        except requests.exceptions.Timeout:
            errors.append("Timeout")
        except requests.exceptions.RequestException as e:
            errors.append(f"Request error: {str(e)}")
        except Exception as e:
            errors.append(f"Error: {str(e)}")
        
        # Small delay between requests
        time.sleep(0.1)
    
    success_rate = len(latencies) / num_tests * 100
    
    result = {
        'chain': chain,
        'name': endpoint['name'],
        'url': endpoint['url'],
        'success_rate': success_rate,
        'errors': errors,
        'block_number': block_number
    }
    
    if latencies:
        result.update({
            'avg_latency': statistics.mean(latencies),
            'min_latency': min(latencies),
            'max_latency': max(latencies),
            'median_latency': statistics.median(latencies),
            'std_latency': statistics.stdev(latencies) if len(latencies) > 1 else 0
        })
    else:
        result.update({
            'avg_latency': None,
            'min_latency': None,
            'max_latency': None,
            'median_latency': None,
            'std_latency': None
        })
    
    return result

def format_latency(latency: float) -> str:
    """Format latency with color coding"""
    if latency is None:
        return "❌ FAILED"
    elif latency < 100:
        return f"🟢 {latency:.0f}ms"
    elif latency < 300:
        return f"🟡 {latency:.0f}ms"
    elif latency < 500:
        return f"🟠 {latency:.0f}ms"
    else:
        return f"🔴 {latency:.0f}ms"

def print_results(results: List[Dict], show_details: bool = False):
    """Print formatted results"""
    if not results:
        print("No results to display")
        return
    
    # Group by chain
    by_chain = {}
    for result in results:
        chain = result['chain']
        if chain not in by_chain:
            by_chain[chain] = []
        by_chain[chain].append(result)
    
    # Print results by chain
    for chain, chain_results in by_chain.items():
        print(f"\n{'='*60}")
        print(f"🔗 {chain.upper()} RESULTS")
        print(f"{'='*60}")
        
        # Sort by average latency
        chain_results.sort(key=lambda x: x['avg_latency'] if x['avg_latency'] is not None else float('inf'))
        
        for i, result in enumerate(chain_results, 1):
            print(f"\n{i}. {result['name']}")
            print(f"   URL: {result['url']}")
            print(f"   Latency: {format_latency(result['avg_latency'])}")
            print(f"   Success Rate: {result['success_rate']:.0f}%")
            if result['block_number']:
                print(f"   Latest Block: {result['block_number']:,}")
            
            if show_details and result['avg_latency'] is not None:
                print(f"   Details: Min={result['min_latency']:.0f}ms, "
                      f"Max={result['max_latency']:.0f}ms, "
                      f"Median={result['median_latency']:.0f}ms")
            
            if result['errors']:
                print(f"   Errors: {', '.join(result['errors'][:3])}")

def print_top_performers(results: List[Dict], top_n: int = 5):
    """Print top performing endpoints across all chains"""
    successful_results = [r for r in results if r['avg_latency'] is not None]
    successful_results.sort(key=lambda x: x['avg_latency'])
    
    print(f"\n{'='*60}")
    print(f"🏆 TOP {min(top_n, len(successful_results))} FASTEST ENDPOINTS (ALL CHAINS)")
    print(f"{'='*60}")
    
    for i, result in enumerate(successful_results[:top_n], 1):
        print(f"\n{i}. {result['name']} ({result['chain'].upper()})")
        print(f"   {format_latency(result['avg_latency'])} | {result['success_rate']:.0f}% success")
        print(f"   {result['url']}")

def main():
    parser = argparse.ArgumentParser(description='Test RPC endpoint latency')
    parser.add_argument('--chains', nargs='+', choices=list(ENDPOINTS.keys()) + ['all'], 
                        default=['all'], help='Chains to test (default: all)')
    parser.add_argument('--tests', type=int, default=5, 
                        help='Number of tests per endpoint (default: 5)')
    parser.add_argument('--details', action='store_true', 
                        help='Show detailed latency statistics')
    parser.add_argument('--top', type=int, default=5, 
                        help='Number of top performers to show (default: 5)')
    parser.add_argument('--parallel', action='store_true', 
                        help='Run tests in parallel (faster but more aggressive)')
    
    args = parser.parse_args()
    
    # Determine which chains to test
    if 'all' in args.chains:
        chains_to_test = list(ENDPOINTS.keys())
    else:
        chains_to_test = args.chains
    
    print("🚀 Starting RPC Endpoint Latency Tests")
    print(f"📊 Testing {sum(len(ENDPOINTS[chain]) for chain in chains_to_test)} endpoints")
    print(f"🔄 {args.tests} tests per endpoint")
    print(f"⛓️  Chains: {', '.join(chains_to_test)}")
    
    # Collect all endpoint tests
    all_tests = []
    for chain in chains_to_test:
        for endpoint in ENDPOINTS[chain]:
            all_tests.append((endpoint, chain, args.tests))
    
    results = []
    
    if args.parallel:
        print("\n⚡ Running tests in parallel...")
        with ThreadPoolExecutor(max_workers=10) as executor:
            future_to_test = {
                executor.submit(test_endpoint, endpoint, chain, num_tests): (endpoint, chain)
                for endpoint, chain, num_tests in all_tests
            }
            
            for i, future in enumerate(as_completed(future_to_test), 1):
                endpoint, chain = future_to_test[future]
                try:
                    result = future.result()
                    results.append(result)
                    print(f"✅ {i}/{len(all_tests)} - {chain.upper()} {endpoint['name']}: "
                          f"{format_latency(result['avg_latency'])}")
                except Exception as e:
                    print(f"❌ {i}/{len(all_tests)} - {chain.upper()} {endpoint['name']}: Error - {e}")
    else:
        print("\n🔄 Running tests sequentially...")
        for i, (endpoint, chain, num_tests) in enumerate(all_tests, 1):
            print(f"Testing {i}/{len(all_tests)}: {chain.upper()} - {endpoint['name']}")
            try:
                result = test_endpoint(endpoint, chain, num_tests)
                results.append(result)
                print(f"  Result: {format_latency(result['avg_latency'])}")
            except Exception as e:
                print(f"  Error: {e}")
            
            # Small delay between endpoints when running sequentially
            time.sleep(0.2)
    
    # Print results
    print_top_performers(results, args.top)
    print_results(results, args.details)
    
    # Save results to JSON
    timestamp = int(time.time())
    filename = f"rpc_latency_results_{timestamp}.json"
    with open(filename, 'w') as f:
        json.dump(results, f, indent=2, default=str)
    
    print(f"\n💾 Results saved to: {filename}")
    print(f"📊 Tested {len(results)} endpoints")
    successful = len([r for r in results if r['avg_latency'] is not None])
    print(f"✅ {successful}/{len(results)} endpoints successful")

if __name__ == "__main__":
    main()
