# 🧪 Mumbai Testnet Testing Plan - Live Arbitrage Validation

## 🎯 **Objective**: Test complete arbitrage flow with real Huff contracts on Mumbai testnet

---

## 📋 **Phase 1: Mumbai Deployment & Setup**

### **1.1 Get Mumbai Test Funds**
```bash
# Get test MATIC from faucet
curl -X POST https://faucet.polygon.technology/mumbai \
  -H "Content-Type: application/json" \
  -d '{"address": "YOUR_ADDRESS"}'

# Or use: https://faucet.polygon.technology/
```

### **1.2 Deploy Huff Contracts to Mumbai**
```bash
# Deploy all three Huff contracts
forge script script/DeployHuffToMumbai.s.sol:DeployHuffToMumbai \
  --rpc-url https://rpc-mumbai.maticvigil.com \
  --private-key $PRIVATE_KEY \
  --broadcast \
  --verify
```

### **1.3 Mumbai Contract Addresses**
```
Mumbai Testnet Addresses:
- USDC: 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174
- WMATIC: 0x9c3C9283D3e44854697Cd22D3Faa240Cfb032889  
- WETH: 0xA6FA4fB5f76172d178d61B04b0ecd319C5d1C0aa

Mumbai DEX Routers:
- QuickSwap: 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff
- SushiSwap: 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506

Mumbai Aave Pool: 0x9198F13B08E299d85E096929fA9781A1E3d5d827
```

---

## 📊 **Phase 2: Real Arbitrage Scanner Setup**

### **2.1 Mumbai Price Monitoring**
```rust
// Rust scanner for Mumbai testnet
pub struct MumbaiArbitrageScanner {
    quickswap_client: DexClient,
    sushiswap_client: DexClient,
    huff_contracts: HuffContracts,
}

impl MumbaiArbitrageScanner {
    pub async fn scan_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        // Monitor key pairs on Mumbai:
        let pairs = vec![
            ("USDC", "WMATIC"),
            ("WMATIC", "WETH"),
            ("USDC", "WETH"),
        ];
        
        for (token_a, token_b) in pairs {
            let quickswap_price = self.get_price("quickswap", token_a, token_b).await;
            let sushiswap_price = self.get_price("sushiswap", token_a, token_b).await;
            
            if let Some(opportunity) = self.calculate_arbitrage(
                quickswap_price, 
                sushiswap_price
            ) {
                opportunities.push(opportunity);
            }
        }
        
        opportunities
    }
}
```

### **2.2 Mumbai Test Token Setup**
```solidity
// Get test tokens on Mumbai
contract MumbaiTestSetup {
    function getTestTokens() external {
        // Request from Mumbai faucets
        IERC20(USDC).transfer(msg.sender, 1000 * 1e6);   // 1000 USDC
        IERC20(WMATIC).transfer(msg.sender, 100 * 1e18); // 100 WMATIC
        IERC20(WETH).transfer(msg.sender, 1 * 1e18);     // 1 WETH
    }
}
```

---

## 🔬 **Phase 3: Live Arbitrage Testing**

### **3.1 End-to-End Test Script**
```bash
#!/bin/bash
# mumbai_arbitrage_test.sh

echo "🧪 Starting Mumbai Arbitrage Testing..."

# 1. Check deployed contract addresses
echo "📍 Checking contract deployments..."
cast code $HUFF_MEV_ADDRESS --rpc-url $MUMBAI_RPC

# 2. Get test tokens
echo "💰 Getting test tokens..."
cast send $USDC_FAUCET "mint(address,uint256)" $WALLET_ADDRESS 1000000000 \
  --rpc-url $MUMBAI_RPC --private-key $PRIVATE_KEY

# 3. Check for arbitrage opportunities
echo "🔍 Scanning for opportunities..."
node scripts/mumbai_scanner.js

# 4. Execute test arbitrage
echo "⚡ Executing test arbitrage..."
cast send $HUFF_MEV_ADDRESS "executeArbitrage(uint256,uint8,bytes)" \
  100000000 1 0x... \
  --rpc-url $MUMBAI_RPC --private-key $PRIVATE_KEY

# 5. Measure gas usage
echo "📊 Measuring gas usage..."
cast receipt $TX_HASH --rpc-url $MUMBAI_RPC
```

### **3.2 Automated Opportunity Detection**
```javascript
// mumbai_scanner.js
const { ethers } = require('ethers');

class MumbaiScanner {
    constructor() {
        this.provider = new ethers.providers.JsonRpcProvider('https://rpc-mumbai.maticvigil.com');
        this.quickswapRouter = '0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff';
        this.sushiswapRouter = '0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506';
    }
    
    async scanArbitrageOpportunities() {
        const pairs = [
            { tokenA: 'USDC', tokenB: 'WMATIC', amount: '1000000000' }, // 1000 USDC
            { tokenA: 'WMATIC', tokenB: 'WETH', amount: '100000000000000000000' }, // 100 WMATIC
        ];
        
        for (const pair of pairs) {
            const quickswapPrice = await this.getPrice('quickswap', pair);
            const sushiswapPrice = await this.getPrice('sushiswap', pair);
            
            const spread = Math.abs(quickswapPrice - sushiswapPrice) / quickswapPrice;
            
            if (spread > 0.005) { // 0.5% minimum spread
                console.log(`🎯 Opportunity found: ${pair.tokenA}/${pair.tokenB}`);
                console.log(`   QuickSwap: ${quickswapPrice}`);
                console.log(`   SushiSwap: ${sushiswapPrice}`);
                console.log(`   Spread: ${(spread * 100).toFixed(2)}%`);
                
                await this.executeArbitrage(pair, spread);
            }
        }
    }
    
    async executeArbitrage(pair, spread) {
        // Build swap data for Huff contract
        const swapData = this.buildSwapData(pair);
        
        // Execute with Huff MEV contract
        const tx = await this.huffMEVContract.executeArbitrage(
            pair.amount,
            2, // numSwaps
            swapData,
            { gasLimit: 500000 }
        );
        
        const receipt = await tx.wait();
        console.log(`✅ Arbitrage executed: ${receipt.transactionHash}`);
        console.log(`⛽ Gas used: ${receipt.gasUsed.toString()}`);
        
        return {
            success: receipt.status === 1,
            gasUsed: receipt.gasUsed.toNumber(),
            txHash: receipt.transactionHash
        };
    }
}
```

---

## 📈 **Phase 4: Performance Validation**

### **4.1 Gas Usage Comparison**
Test all three contracts with identical arbitrage opportunities:

| Contract | Expected Gas | Actual Gas | Difference |
|----------|--------------|------------|------------|
| Huff Extreme | ~3,800 | TBD | TBD |
| Huff MEV | ~3,800 | TBD | TBD |
| Huff Ultra | ~3,800 | TBD | TBD |

### **4.2 Success Rate Testing**
```bash
# Run 100 test arbitrages
for i in {1..100}; do
    echo "Test $i/100"
    ./execute_test_arbitrage.sh
    sleep 10
done

# Calculate success rate
echo "Success rate: $(grep "SUCCESS" results.log | wc -l)%"
```

### **4.3 Profitability Analysis**
```rust
// Real Mumbai profitability calculation
pub fn analyze_mumbai_results(results: &[ArbitrageResult]) -> ProfitabilityReport {
    let total_gas_cost: f64 = results.iter()
        .map(|r| r.gas_used as f64 * 30e9 * 0.8 / 1e18) // 30 gwei, $0.8 MATIC
        .sum();
    
    let total_profit: f64 = results.iter()
        .map(|r| r.profit_usd)
        .sum();
    
    ProfitabilityReport {
        total_arbitrages: results.len(),
        success_rate: results.iter().filter(|r| r.success).count() as f64 / results.len() as f64,
        total_profit_usd: total_profit,
        total_gas_cost_usd: total_gas_cost,
        net_profit_usd: total_profit - total_gas_cost,
        average_gas_per_arbitrage: results.iter().map(|r| r.gas_used).sum::<u64>() / results.len() as u64,
    }
}
```

---

## 🎯 **Phase 5: Mainnet Preparation**

### **5.1 Mumbai Test Results → Mainnet Strategy**
Based on Mumbai results:
1. **Optimal contract selection** (Extreme vs MEV vs Ultra)
2. **Gas usage patterns** under real conditions
3. **Success rate** with actual DEX liquidity
4. **Profitability thresholds** for different market conditions

### **5.2 Mainnet Deployment Plan**
```rust
// Production deployment strategy based on Mumbai results
pub enum MainnetStrategy {
    ConservativeStart {
        contract: ContractType::HuffMEV,
        min_profit_threshold: 5.0, // $5 minimum
        max_gas_price: 100, // 100 gwei limit
    },
    AggressiveOptimization {
        contract: ContractType::HuffUltra,
        min_profit_threshold: 1.0, // $1 minimum
        max_gas_price: 200, // 200 gwei limit
    },
}
```

---

## 🚀 **Expected Mumbai Results**

### **Success Criteria:**
1. ✅ **All contracts deploy** successfully
2. ✅ **Gas usage < 5,000** for simple arbitrages
3. ✅ **Success rate > 80%** for detected opportunities
4. ✅ **Net profitability** after gas costs
5. ✅ **Ultra contract shows advantages** for complex routes

### **Key Metrics to Measure:**
- **Actual execution gas** (vs our estimates)
- **Transaction success rate**
- **Time to execution** (block confirmation)
- **Slippage impact** on profitability
- **Gas price impact** on viability

---

## 📋 **Next Steps**

1. **Get Mumbai MATIC** from faucets
2. **Deploy contracts** to Mumbai testnet
3. **Setup price monitoring** for Mumbai DEXs
4. **Run automated scanner** for 24-48 hours
5. **Execute test arbitrages** and measure performance
6. **Analyze results** and optimize for mainnet

**This Mumbai testing will give us REAL performance data for mainnet deployment strategy!** 🚀