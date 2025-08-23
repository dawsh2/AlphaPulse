# Flashbots Integration Guide

## Overview

Flashbots provides private transaction pools and MEV protection through sealed-bid auctions, preventing frontrunning and sandwich attacks while enabling priority execution for our arbitrage trades.

## Core Concepts

### What Flashbots Provides
1. **Private Mempool**: Transactions aren't visible until mined
2. **Bundle Auctions**: Compete on bundle profitability, not gas wars
3. **No Failed Tx Costs**: Only pay if bundle is included
4. **Atomic Execution**: All-or-nothing bundle execution

## Implementation

### 1. Flashbots Bundle Creation

```rust
use ethers_flashbots::{
    FlashbotsMiddleware, 
    BundleRequest, 
    SimulatedBundle,
    BundleTransaction
};

pub struct FlashbotsExecutor {
    client: FlashbotsMiddleware<Provider<Http>, LocalWallet>,
    signer: LocalWallet,
    relay_url: String,
    
    pub async fn execute_arbitrage_bundle(
        &self,
        arbitrage_tx: Transaction,
        bribe_percentage: f64  // Percentage of profit to pay miners
    ) -> Result<H256> {
        // Calculate expected profit
        let expected_profit = self.simulate_profit(&arbitrage_tx).await?;
        
        // Create bundle with bribe
        let miner_bribe = (expected_profit * bribe_percentage) as u128;
        
        let bundle = BundleRequest::new()
            .set_block_number(self.get_next_block().await?)
            .push_transaction(arbitrage_tx)
            .push_transaction(self.create_bribe_tx(miner_bribe).await?);
        
        // Simulate bundle first
        let simulation = self.client.simulate_bundle(&bundle).await?;
        
        if !self.is_simulation_profitable(&simulation) {
            return Err(Error::UnprofitableBundle);
        }
        
        // Send bundle
        let pending_bundle = self.client.send_bundle(&bundle).await?;
        
        // Wait for inclusion
        self.wait_for_inclusion(pending_bundle).await
    }
    
    fn create_bribe_tx(&self, amount: u128) -> Transaction {
        // Direct transfer to block.coinbase
        Transaction {
            to: Some(Address::zero()), // Will be replaced with block.coinbase
            value: Some(U256::from(amount)),
            data: vec![],
            gas: 21000,
            ..Default::default()
        }
    }
    
    async fn wait_for_inclusion(&self, bundle: PendingBundle) -> Result<H256> {
        let mut attempts = 0;
        let max_attempts = 25; // ~25 blocks
        
        while attempts < max_attempts {
            match bundle.status().await {
                Ok(BundleStatus::Included(block)) => {
                    return Ok(block.hash);
                },
                Ok(BundleStatus::Rejected(reason)) => {
                    return Err(Error::BundleRejected(reason));
                },
                Ok(BundleStatus::Pending) => {
                    // Resubmit for next block
                    self.resubmit_bundle(&bundle).await?;
                },
                Err(e) => return Err(Error::from(e))
            }
            
            attempts += 1;
            sleep(Duration::from_secs(1)).await;
        }
        
        Err(Error::BundleTimeout)
    }
}
```

### 2. Multi-Transaction Bundles

```typescript
class FlashbotsBundleBuilder {
    /**
     * Build complex arbitrage bundles with multiple transactions
     */
    async buildArbitrageBundle(
        opportunity: ArbitrageOpportunity
    ): Promise<FlashbotsBundle> {
        const transactions: BundleTransaction[] = [];
        
        // Transaction 1: Flash loan initiation (if needed)
        if (opportunity.requiresFlashLoan) {
            transactions.push({
                transaction: await this.createFlashLoanTx(opportunity),
                signer: this.arbitrageSigner
            });
        }
        
        // Transaction 2: Main arbitrage execution
        transactions.push({
            transaction: await this.createArbitrageTx(opportunity),
            signer: this.arbitrageSigner
        });
        
        // Transaction 3: Bribe to miner (incentive for inclusion)
        const bribe = this.calculateOptimalBribe(opportunity);
        transactions.push({
            transaction: {
                to: "0x0000000000000000000000000000000000000000",
                value: ethers.utils.parseEther(bribe.toString()),
                data: "0x",
                gasLimit: 21000,
                chainId: 137, // Polygon
                type: 2,
                maxFeePerGas: opportunity.gasPrice,
                maxPriorityFeePerGas: opportunity.gasPrice
            },
            signer: this.bribeSigner
        });
        
        return {
            transactions,
            blockNumber: await this.provider.getBlockNumber() + 1,
            minTimestamp: Math.floor(Date.now() / 1000),
            maxTimestamp: Math.floor(Date.now() / 1000) + 120,
            revertingTxHashes: [] // All txs must succeed
        };
    }
    
    calculateOptimalBribe(opportunity: ArbitrageOpportunity): number {
        /**
         * Dynamic bribe calculation based on:
         * - Expected profit
         * - Competition level
         * - Network congestion
         */
        
        const baseProfit = opportunity.expectedProfit;
        const competitionMultiplier = this.getCompetitionLevel();
        const congestionMultiplier = this.getNetworkCongestion();
        
        // Start with 10% of profit as base bribe
        let bribe = baseProfit * 0.1;
        
        // Adjust for competition (up to 50% in high competition)
        bribe *= (1 + competitionMultiplier * 0.4);
        
        // Adjust for congestion (up to 30% extra in congestion)
        bribe *= (1 + congestionMultiplier * 0.3);
        
        // Cap at 80% of profit
        return Math.min(bribe, baseProfit * 0.8);
    }
}
```

### 3. Polygon Flashbots (via Marlin)

```python
class PolygonFlashbots:
    """
    Flashbots-style private mempool for Polygon via Marlin
    """
    
    def __init__(self):
        self.relay_url = "https://polygon-relay.marlin.org"
        self.rpc_url = "https://polygon-rpc.marlin.org"
        self.signer = Account.from_key(os.environ['FLASHBOTS_SIGNER_KEY'])
        
    async def send_private_transaction(self, tx: dict) -> str:
        """Send transaction through Marlin's private pool"""
        
        # Sign transaction
        signed_tx = self.w3.eth.account.sign_transaction(tx, self.signer.key)
        
        # Create bundle
        bundle = {
            "jsonrpc": "2.0",
            "method": "eth_sendBundle",
            "params": [{
                "txs": [signed_tx.rawTransaction.hex()],
                "blockNumber": hex(await self.get_next_block()),
                "minTimestamp": 0,
                "maxTimestamp": int(time.time()) + 120
            }],
            "id": 1
        }
        
        # Sign bundle
        bundle_hash = self.hash_bundle(bundle)
        signature = self.signer.signHash(bundle_hash)
        
        headers = {
            "X-Flashbots-Signature": f"{self.signer.address}:{signature.signature.hex()}"
        }
        
        # Send to relay
        response = await aiohttp.post(
            self.relay_url,
            json=bundle,
            headers=headers
        )
        
        result = await response.json()
        
        if 'error' in result:
            raise FlashbotsError(result['error'])
            
        return result['result']['bundleHash']
    
    async def simulate_bundle(self, transactions: List[dict]) -> dict:
        """Simulate bundle execution"""
        
        params = {
            "txs": [self.sign_tx(tx) for tx in transactions],
            "blockNumber": hex(await self.get_next_block()),
            "stateBlockNumber": "latest"
        }
        
        response = await self.rpc_call("eth_callBundle", params)
        
        return {
            "success": response.get("results", [{}])[0].get("error") is None,
            "profit": self.calculate_bundle_profit(response),
            "gas_used": sum(r.get("gasUsed", 0) for r in response.get("results", [])),
            "revert_reason": response.get("results", [{}])[0].get("error")
        }
```

### 4. Bundle Optimization Strategies

```rust
pub struct BundleOptimizer {
    pub fn optimize_bundle_for_inclusion(&self, opportunity: &Opportunity) -> Bundle {
        // Strategy 1: Target specific block slots
        let target_block = self.predict_low_competition_block();
        
        // Strategy 2: Dynamic bribe adjustment
        let optimal_bribe = self.calculate_dynamic_bribe(
            opportunity.profit,
            self.get_current_competition_level(),
            self.get_gas_price_percentile(90)
        );
        
        // Strategy 3: Multi-block targeting
        let blocks_to_target = if opportunity.profit > 1000 {
            vec![target_block, target_block + 1, target_block + 2]
        } else {
            vec![target_block]
        };
        
        // Strategy 4: Decoy transactions
        let bundle = if self.detect_bundle_competition() {
            self.add_decoy_transactions(opportunity)
        } else {
            self.create_simple_bundle(opportunity)
        };
        
        bundle
    }
    
    fn add_decoy_transactions(&self, opportunity: &Opportunity) -> Bundle {
        // Add meaningless transactions to obscure intent
        let mut bundle = Bundle::new();
        
        // Decoy 1: Small token transfer
        bundle.add_transaction(self.create_decoy_transfer());
        
        // Real transaction in the middle
        bundle.add_transaction(opportunity.to_transaction());
        
        // Decoy 2: Another small operation
        bundle.add_transaction(self.create_decoy_interaction());
        
        // Miner bribe at the end
        bundle.add_transaction(self.create_bribe_transaction());
        
        bundle
    }
}
```

### 5. Fallback Strategies

```typescript
class FlashbotsWithFallback {
    async executeWithProtection(tx: Transaction): Promise<Receipt> {
        const strategies = [
            () => this.tryFlashbotsBundle(tx),
            () => this.tryMarlinRelay(tx),
            () => this.tryEdenNetwork(tx),
            () => this.trySecureRPC(tx),
            () => this.tryCommitReveal(tx),
            () => this.tryWithMaxGasProtection(tx)
        ];
        
        for (const [index, strategy] of strategies.entries()) {
            try {
                console.log(`Attempting strategy ${index + 1}/${strategies.length}`);
                const receipt = await strategy();
                
                if (receipt && receipt.status === 1) {
                    console.log(`Success with strategy ${index + 1}`);
                    return receipt;
                }
            } catch (error) {
                console.log(`Strategy ${index + 1} failed:`, error.message);
                continue;
            }
        }
        
        throw new Error("All protection strategies failed");
    }
    
    async tryFlashbotsBundle(tx: Transaction): Promise<Receipt> {
        const bundle = await this.buildBundle(tx);
        const result = await this.flashbotsProvider.sendBundle(bundle);
        
        // Wait up to 5 blocks
        for (let i = 0; i < 5; i++) {
            const status = await result.wait();
            if (status === BundleStatus.Included) {
                return await this.getReceipt(tx.hash);
            }
            
            // Resubmit for next block
            await result.resubmit();
        }
        
        throw new Error("Bundle not included after 5 blocks");
    }
}
```

## Monitoring & Analytics

### Bundle Performance Tracking

```python
class FlashbotsAnalytics:
    def track_bundle_performance(self):
        return {
            "total_bundles_sent": self.total_sent,
            "inclusion_rate": self.included / self.total_sent,
            "average_blocks_to_inclusion": self.avg_blocks_to_inclusion,
            "total_bribes_paid": self.total_bribes,
            "average_bribe_percentage": self.avg_bribe_percentage,
            "profit_after_bribes": self.total_profit - self.total_bribes,
            "failed_simulations": self.failed_sims,
            "rejected_bundles": self.rejected,
            "timeout_bundles": self.timeouts,
            "competition_encounters": self.competition_detected
        }
```

### Real-Time Monitoring

```rust
pub struct FlashbotsMonitor {
    pub async fn monitor_bundle_status(&self, bundle_hash: H256) {
        loop {
            let status = self.check_bundle_status(bundle_hash).await;
            
            match status {
                BundleStatus::Pending => {
                    println!("Bundle pending in block {}", self.current_block());
                },
                BundleStatus::Included(block) => {
                    println!("Bundle included in block {}", block);
                    self.record_success(bundle_hash, block);
                    break;
                },
                BundleStatus::Failed(reason) => {
                    println!("Bundle failed: {}", reason);
                    self.record_failure(bundle_hash, reason);
                    break;
                }
            }
            
            sleep(Duration::from_secs(1)).await;
        }
    }
}
```

## Best Practices

### 1. Bribe Optimization
- Start with 10% of expected profit
- Increase during high competition
- Cap at 50% to maintain profitability

### 2. Bundle Construction
- Keep bundles simple (2-3 transactions)
- Always include miner payment
- Use atomic transactions when possible

### 3. Timing
- Target next 1-3 blocks
- Resubmit if not included
- Have fallback strategies ready

### 4. Security
- Never expose signing keys
- Use separate wallets for bribes
- Monitor for bundle copying

## Troubleshooting

### Bundle Not Included
1. Increase bribe amount
2. Check simulation results
3. Verify gas prices
4. Try different target blocks

### High Rejection Rate
1. Improve profit calculation
2. Reduce bundle complexity
3. Check for reverts in simulation
4. Verify contract interactions

### Competition Detection
1. Monitor other bundles
2. Randomize strategies
3. Use decoy transactions
4. Switch to backup providers

## Conclusion

Flashbots integration provides essential MEV protection for arbitrage operations. By keeping transactions private until execution and using bundle auctions instead of gas wars, we can execute profitable trades without being frontrun or sandwiched. The key is balancing bribe amounts with profitability while maintaining multiple fallback options.