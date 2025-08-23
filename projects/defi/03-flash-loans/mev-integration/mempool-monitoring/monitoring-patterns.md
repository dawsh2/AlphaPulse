# Monitoring Patterns - MEV Detection & Opportunity Identification

## Pattern 1: Large Swap Detection 🐋

### Pattern Recognition
```python
class LargeSwapDetector:
    def __init__(self):
        self.threshold_usd = 10000  # $10k minimum
        self.price_feeds = PriceFeedManager()
        
    def detect_large_swap(self, tx):
        """Identify large swaps that will move the market"""
        if not self.is_dex_router(tx['to']):
            return None
            
        decoded = self.decode_swap_data(tx['input'])
        if not decoded:
            return None
        
        # Calculate USD value
        token_price = self.price_feeds.get_price(decoded['token_in'])
        swap_value_usd = decoded['amount_in'] * token_price
        
        if swap_value_usd > self.threshold_usd:
            return {
                'type': 'LARGE_SWAP',
                'tx_hash': tx['hash'],
                'value_usd': swap_value_usd,
                'token_in': decoded['token_in'],
                'token_out': decoded['token_out'],
                'expected_impact': self.calculate_price_impact(decoded),
                'opportunity': self.identify_arbitrage_opportunity(decoded)
            }
```

### Action Strategy
```python
def execute_large_swap_strategy(large_swap):
    """Execute strategy based on large swap detection"""
    impact = large_swap['expected_impact']
    
    if impact > 0.01:  # 1% price impact
        # Front-run with smaller trade
        return {
            'action': 'FRONT_RUN',
            'size': large_swap['value_usd'] * 0.1,  # 10% of whale trade
            'direction': 'SAME',
            'gas_premium': 2  # 2x gas to ensure priority
        }
    elif impact < -0.01:
        # Trade opposite direction after whale
        return {
            'action': 'COUNTER_TRADE',
            'size': large_swap['value_usd'] * 0.2,
            'direction': 'OPPOSITE',
            'timing': 'AFTER'
        }
```

## Pattern 2: Sandwich Attack Opportunities 🥪

### Victim Identification
```python
class SandwichScanner:
    def identify_victims(self, mempool):
        """Find transactions vulnerable to sandwiching"""
        victims = []
        
        for tx in mempool:
            if self.is_swap(tx):
                slippage = self.extract_slippage_tolerance(tx)
                
                if slippage > 0.005:  # >0.5% slippage tolerance
                    profit = self.calculate_sandwich_profit(tx, slippage)
                    
                    if profit > 50:  # $50 minimum profit
                        victims.append({
                            'tx': tx,
                            'slippage': slippage,
                            'expected_profit': profit,
                            'front_params': self.calculate_front_run(tx),
                            'back_params': self.calculate_back_run(tx)
                        })
        
        return sorted(victims, key=lambda x: x['expected_profit'], reverse=True)
    
    def calculate_sandwich_profit(self, victim_tx, slippage):
        """Calculate expected profit from sandwich attack"""
        # Simulate front-run transaction
        front_run_impact = self.simulate_trade(
            victim_tx['token_in'],
            victim_tx['amount'] * 0.1  # 10% of victim size
        )
        
        # Victim executes at worse price
        victim_execution_price = self.current_price * (1 + front_run_impact)
        victim_received = victim_tx['amount'] / victim_execution_price
        victim_slippage_loss = victim_tx['expected_out'] - victim_received
        
        # Back-run to sell
        back_run_price = victim_execution_price * (1 + victim_tx['impact'])
        back_run_profit = (back_run_price - self.current_price) * front_run_size
        
        # Net profit after gas
        gas_cost = self.estimate_gas_cost() * 2  # Two transactions
        
        return back_run_profit - gas_cost
```

### Execution Pattern
```python
async def execute_sandwich(victim):
    """Execute sandwich attack"""
    # Transaction 1: Front-run
    front_tx = {
        'to': ROUTER_ADDRESS,
        'data': encode_swap(
            token_in=victim['token_in'],
            token_out=victim['token_out'],
            amount=victim['front_params']['amount']
        ),
        'gasPrice': victim['gas_price'] + 1,  # Slightly higher gas
        'nonce': get_nonce()
    }
    
    # Transaction 2: Back-run
    back_tx = {
        'to': ROUTER_ADDRESS,
        'data': encode_swap(
            token_in=victim['token_out'],  # Reverse direction
            token_out=victim['token_in'],
            amount=victim['back_params']['amount']
        ),
        'gasPrice': victim['gas_price'] - 1,  # Slightly lower gas
        'nonce': get_nonce() + 1
    }
    
    # Submit both transactions
    bundle = [front_tx, victim['tx'], back_tx]
    return await submit_bundle(bundle)
```

## Pattern 3: Liquidation Hunting 💀

### Position Monitoring
```python
class LiquidationHunter:
    def __init__(self):
        self.lending_protocols = {
            'aave': AaveMonitor(),
            'compound': CompoundMonitor(),
            'maker': MakerMonitor()
        }
    
    def scan_for_liquidations(self, price_impact_tx):
        """Find positions that will be liquidatable after tx"""
        at_risk_positions = []
        
        # Predict price after transaction
        predicted_price = self.predict_price_after_tx(price_impact_tx)
        
        for protocol_name, monitor in self.lending_protocols.items():
            positions = monitor.get_all_positions()
            
            for position in positions:
                # Calculate health factor with new price
                new_health = self.calculate_health_factor(
                    position,
                    predicted_price
                )
                
                if new_health < 1.0:  # Liquidatable
                    profit = self.calculate_liquidation_profit(position)
                    
                    at_risk_positions.append({
                        'protocol': protocol_name,
                        'position': position,
                        'health_factor': new_health,
                        'collateral': position['collateral_value'],
                        'debt': position['debt_value'],
                        'profit': profit,
                        'trigger_tx': price_impact_tx['hash']
                    })
        
        return at_risk_positions
```

### Liquidation Execution
```python
async def execute_liquidation(opportunity):
    """Execute liquidation after trigger transaction"""
    # Wait for trigger transaction to be mined
    await wait_for_tx(opportunity['trigger_tx'])
    
    # Immediately submit liquidation
    liquidation_tx = {
        'to': opportunity['protocol'].liquidation_contract,
        'data': encode_liquidation(
            borrower=opportunity['position']['owner'],
            debt_asset=opportunity['position']['debt_token'],
            collateral_asset=opportunity['position']['collateral_token'],
            debt_to_cover=opportunity['position']['debt_amount']
        ),
        'gasPrice': calculate_competitive_gas(),
        'value': 0
    }
    
    return await send_transaction(liquidation_tx)
```

## Pattern 4: Arbitrage Path Detection 🔄

### Multi-DEX Arbitrage
```python
class ArbitragePathDetector:
    def detect_arbitrage_paths(self, pending_swap):
        """Find arbitrage created by pending swap"""
        paths = []
        
        # Get current prices across all DEXes
        prices_before = self.get_all_dex_prices(
            pending_swap['token_in'],
            pending_swap['token_out']
        )
        
        # Simulate price after swap
        affected_dex = pending_swap['dex']
        new_price = self.simulate_swap_impact(pending_swap)
        
        # Find arbitrage opportunities
        for other_dex in self.dexes:
            if other_dex != affected_dex:
                price_diff = abs(new_price - prices_before[other_dex])
                
                if price_diff > 0.005:  # 0.5% difference
                    profit = self.calculate_arb_profit(
                        affected_dex,
                        other_dex,
                        price_diff,
                        pending_swap['amount']
                    )
                    
                    if profit > 50:  # $50 minimum
                        paths.append({
                            'buy_dex': affected_dex if new_price < prices_before[other_dex] else other_dex,
                            'sell_dex': other_dex if new_price < prices_before[other_dex] else affected_dex,
                            'token_pair': (pending_swap['token_in'], pending_swap['token_out']),
                            'price_diff': price_diff,
                            'profit': profit,
                            'optimal_size': self.calculate_optimal_size(price_diff)
                        })
        
        return paths
```

## Pattern 5: Token Launch Sniping 🚀

### New Pair Detection
```python
class TokenLaunchSniper:
    def detect_new_pairs(self, mempool):
        """Detect new token pair creation"""
        for tx in mempool:
            if self.is_create_pair_tx(tx):
                pair_data = self.decode_create_pair(tx)
                
                return {
                    'type': 'NEW_PAIR',
                    'token0': pair_data['token0'],
                    'token1': pair_data['token1'],
                    'initial_liquidity': pair_data['liquidity'],
                    'creator': tx['from'],
                    'strategy': self.determine_snipe_strategy(pair_data)
                }
    
    def determine_snipe_strategy(self, pair_data):
        """Determine if worth sniping"""
        # Check if legitimate project
        if self.is_honeypot(pair_data['token0']):
            return {'action': 'AVOID', 'reason': 'Honeypot detected'}
        
        # Check liquidity
        if pair_data['initial_liquidity'] < 10000:  # $10k minimum
            return {'action': 'SKIP', 'reason': 'Low liquidity'}
        
        # Check for renounced ownership
        if not self.is_ownership_renounced(pair_data['token0']):
            return {'action': 'WAIT', 'reason': 'Ownership not renounced'}
        
        return {
            'action': 'SNIPE',
            'amount': min(pair_data['initial_liquidity'] * 0.01, 1000),  # 1% or $1000
            'timing': 'IMMEDIATE'
        }
```

## Pattern 6: MEV Bundle Detection 📦

### Bundle Pattern Recognition
```python
class MEVBundleDetector:
    def detect_mev_bundles(self, mempool):
        """Identify coordinated MEV bundles"""
        potential_bundles = []
        
        # Group transactions by sender
        tx_by_sender = defaultdict(list)
        for tx in mempool:
            tx_by_sender[tx['from']].append(tx)
        
        # Look for bundle patterns
        for sender, txs in tx_by_sender.items():
            if len(txs) >= 2:
                # Check if transactions are related
                if self.are_transactions_related(txs):
                    bundle_type = self.classify_bundle(txs)
                    
                    potential_bundles.append({
                        'sender': sender,
                        'transactions': txs,
                        'type': bundle_type,
                        'estimated_profit': self.estimate_bundle_profit(txs),
                        'counter_strategy': self.generate_counter_bundle(txs)
                    })
        
        return potential_bundles
    
    def classify_bundle(self, txs):
        """Classify the type of MEV bundle"""
        # Check for sandwich pattern
        if len(txs) == 3 and self.is_sandwich_pattern(txs):
            return 'SANDWICH'
        
        # Check for liquidation + swap
        if self.has_liquidation(txs) and self.has_swap(txs):
            return 'LIQUIDATION_ARB'
        
        # Check for multi-dex arbitrage
        if self.is_multi_dex_arb(txs):
            return 'MULTI_DEX_ARB'
        
        return 'UNKNOWN'
```

## Pattern 7: Gas War Detection ⛽

### Gas Spike Analysis
```python
class GasWarDetector:
    def detect_gas_wars(self, mempool):
        """Detect competitive gas bidding wars"""
        # Group by same function call
        function_groups = defaultdict(list)
        
        for tx in mempool:
            func_sig = tx['input'][:10] if len(tx['input']) >= 10 else None
            if func_sig:
                function_groups[func_sig].append(tx)
        
        gas_wars = []
        for func_sig, txs in function_groups.items():
            if len(txs) >= 3:  # Multiple parties competing
                gas_prices = [int(tx['gasPrice'], 16) for tx in txs]
                
                if max(gas_prices) > min(gas_prices) * 2:  # 2x difference
                    gas_wars.append({
                        'function': func_sig,
                        'competitors': len(txs),
                        'min_gas': min(gas_prices),
                        'max_gas': max(gas_prices),
                        'opportunity': self.decode_opportunity(txs[0]),
                        'strategy': self.determine_gas_strategy(gas_prices)
                    })
        
        return gas_wars
    
    def determine_gas_strategy(self, gas_prices):
        """Determine optimal gas bidding strategy"""
        avg_gas = sum(gas_prices) / len(gas_prices)
        std_dev = statistics.stdev(gas_prices)
        
        if std_dev > avg_gas * 0.5:  # High variance
            return {
                'action': 'WAIT',
                'reason': 'Gas war too expensive',
                'threshold': avg_gas * 1.5
            }
        else:
            return {
                'action': 'OUTBID',
                'gas_price': max(gas_prices) + 1000000000,  # +1 gwei
                'max_gas': avg_gas * 2  # Don't exceed 2x average
            }
```

## Pattern 8: Flash Loan Detection 🎯

### Flash Loan Activity
```python
class FlashLoanDetector:
    def detect_flash_loans(self, tx):
        """Detect flash loan usage in transactions"""
        # Check for Aave flash loan
        if self.is_aave_flash_loan(tx):
            return self.decode_aave_flash_loan(tx)
        
        # Check for Uniswap flash swap
        if self.is_uniswap_flash_swap(tx):
            return self.decode_uniswap_flash_swap(tx)
        
        # Check for dYdX flash loan
        if self.is_dydx_flash_loan(tx):
            return self.decode_dydx_flash_loan(tx)
        
        return None
    
    def analyze_flash_loan_strategy(self, flash_loan):
        """Determine what the flash loan is being used for"""
        internal_calls = self.trace_internal_calls(flash_loan['tx'])
        
        # Check for arbitrage
        if self.has_multiple_swaps(internal_calls):
            return {
                'type': 'ARBITRAGE',
                'path': self.extract_swap_path(internal_calls),
                'profit': self.calculate_arbitrage_profit(internal_calls)
            }
        
        # Check for liquidation
        if self.has_liquidation_call(internal_calls):
            return {
                'type': 'LIQUIDATION',
                'target': self.extract_liquidation_target(internal_calls),
                'profit': self.calculate_liquidation_profit(internal_calls)
            }
        
        # Check for collateral swap
        if self.has_collateral_swap(internal_calls):
            return {
                'type': 'COLLATERAL_SWAP',
                'protocol': self.identify_lending_protocol(internal_calls)
            }
```

## Real-Time Pattern Matching Engine

### Master Pattern Detector
```python
class PatternEngine:
    def __init__(self):
        self.detectors = [
            LargeSwapDetector(),
            SandwichScanner(),
            LiquidationHunter(),
            ArbitragePathDetector(),
            TokenLaunchSniper(),
            MEVBundleDetector(),
            GasWarDetector(),
            FlashLoanDetector()
        ]
        
    async def analyze_mempool(self, mempool_snapshot):
        """Run all pattern detectors in parallel"""
        tasks = []
        
        for detector in self.detectors:
            tasks.append(detector.analyze(mempool_snapshot))
        
        results = await asyncio.gather(*tasks)
        
        # Combine and prioritize opportunities
        all_opportunities = []
        for detector_results in results:
            if detector_results:
                all_opportunities.extend(detector_results)
        
        # Sort by expected profit
        return sorted(
            all_opportunities,
            key=lambda x: x.get('profit', 0),
            reverse=True
        )
```

## Alert Configuration

### Pattern-Based Alerts
```yaml
alerts:
  large_swap:
    enabled: true
    threshold: 50000  # $50k USD
    notification: webhook
    
  sandwich_opportunity:
    enabled: true
    min_profit: 100  # $100 minimum
    max_gas_ratio: 0.5  # Gas max 50% of profit
    
  liquidation:
    enabled: true
    protocols: ['aave', 'compound']
    min_collateral: 10000  # $10k minimum
    
  new_token_launch:
    enabled: true
    min_liquidity: 25000  # $25k minimum
    honeypot_check: true
    
  gas_war:
    enabled: true
    threshold_multiplier: 3  # 3x normal gas
    
  flash_loan_activity:
    enabled: true
    min_amount: 100000  # $100k minimum
```

## Pattern 9: Post-MEV Cleanup Opportunities 🧹

### MEV Bot Activity Detection
```python
class PostMEVCleanupDetector:
    def __init__(self):
        self.known_mev_bots = self.load_known_mev_addresses()
        self.mev_patterns = self.load_mev_behavior_patterns()
        
    def detect_mev_execution(self, tx):
        """Identify MEV bot transactions that create cleanup opportunities"""
        
        # Check if known MEV bot
        if tx['from'] in self.known_mev_bots:
            return self.analyze_mev_impact(tx)
        
        # Detect MEV patterns
        if self.matches_mev_pattern(tx):
            return self.predict_cleanup_opportunities(tx)
        
        return None
    
    def analyze_mev_impact(self, mev_tx):
        """Analyze MEV transaction for secondary opportunities"""
        
        # Decode MEV bot's trade
        trade_path = self.decode_trade_path(mev_tx)
        impact = self.calculate_market_impact(trade_path)
        
        cleanup_opportunities = []
        
        # Find pools affected but not included in MEV path
        affected_pools = self.find_affected_pools(trade_path)
        missed_pools = self.find_adjacent_pools(trade_path) - set(trade_path.pools)
        
        for pool in missed_pools:
            # Calculate expected inefficiency
            expected_imbalance = self.calculate_ripple_effect(
                pool, 
                impact,
                trade_path
            )
            
            if expected_imbalance > 0.002:  # 0.2% opportunity
                cleanup_opportunities.append({
                    'type': 'POST_MEV_CLEANUP',
                    'trigger_tx': mev_tx['hash'],
                    'pool': pool,
                    'expected_profit': self.estimate_cleanup_profit(pool, expected_imbalance),
                    'timing': 'AFTER_CONFIRMATION',
                    'competition_level': 'LOW'  # Key advantage
                })
        
        return cleanup_opportunities
    
    def predict_cleanup_opportunities(self, mev_tx):
        """Predict where MEV bot will create inefficiencies"""
        
        opportunities = []
        
        # Overcorrection opportunities
        if self.is_large_arbitrage(mev_tx):
            overcorrection = self.predict_overcorrection(mev_tx)
            if overcorrection:
                opportunities.append({
                    'type': 'OVERCORRECTION_CLEANUP',
                    'pools': overcorrection['affected_pools'],
                    'direction': 'REVERSE',
                    'expected_profit': overcorrection['profit']
                })
        
        # Cascade opportunities
        cascade_effects = self.predict_cascade_effects(mev_tx)
        for effect in cascade_effects:
            opportunities.append({
                'type': 'CASCADE_CLEANUP',
                'sequence': effect['pool_sequence'],
                'total_profit': effect['cumulative_profit']
            })
        
        # Liquidation aftermath
        if self.is_liquidation(mev_tx):
            aftermath = self.predict_liquidation_aftermath(mev_tx)
            opportunities.append({
                'type': 'LIQUIDATION_CLEANUP',
                'collateral_discount': aftermath['collateral_opportunity'],
                'debt_premium': aftermath['debt_opportunity']
            })
        
        return opportunities
```

### Cleanup Execution Strategy
```python
class CleanupExecutor:
    async def execute_post_mev_cleanup(self, mev_detection):
        """Execute cleanup strategy after MEV bot"""
        
        # Wait for MEV transaction confirmation
        await self.wait_for_confirmation(mev_detection['trigger_tx'])
        
        # Immediately scan actual impact
        actual_impact = self.measure_actual_impact(mev_detection['pools'])
        
        # Execute cleanup trades
        cleanup_trades = []
        
        for opportunity in mev_detection['opportunities']:
            if self.is_still_profitable(opportunity, actual_impact):
                trade = self.build_cleanup_trade(opportunity)
                
                # Use moderate gas (no competition)
                trade['gasPrice'] = self.get_percentile_gas(50)  # 50th percentile
                
                result = await self.execute_trade(trade)
                cleanup_trades.append(result)
        
        return {
            'mev_tx': mev_detection['trigger_tx'],
            'cleanup_count': len(cleanup_trades),
            'total_profit': sum(t['profit'] for t in cleanup_trades),
            'gas_saved': self.calculate_gas_savings(cleanup_trades)  # No bidding war
        }
```

## Performance Metrics

### Pattern Detection Statistics
```python
def calculate_pattern_metrics(detected_patterns, actual_outcomes):
    """Calculate accuracy and profit metrics"""
    metrics = {}
    
    for pattern_type in PATTERN_TYPES:
        pattern_data = filter_by_type(detected_patterns, pattern_type)
        outcomes = filter_by_type(actual_outcomes, pattern_type)
        
        metrics[pattern_type] = {
            'detection_rate': len(pattern_data) / len(outcomes),
            'false_positive_rate': calculate_false_positives(pattern_data, outcomes),
            'avg_profit': calculate_average_profit(outcomes),
            'success_rate': calculate_success_rate(outcomes),
            'avg_detection_time': calculate_avg_detection_time(pattern_data)
        }
    
    # Special metrics for post-MEV cleanup
    if 'POST_MEV_CLEANUP' in metrics:
        metrics['POST_MEV_CLEANUP']['competition_rate'] = calculate_competition_rate()
        metrics['POST_MEV_CLEANUP']['gas_efficiency'] = calculate_gas_efficiency()
    
    return metrics
```