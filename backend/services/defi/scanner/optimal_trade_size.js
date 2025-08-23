const { Web3 } = require('web3');
const web3 = new Web3('https://polygon-rpc.com');

// Calculate optimal trade size dynamically
function calculateOptimalTradeSize(buyPool, sellPool, gasUSD = 0.50) {
    const { reserve0: buyR0, reserve1: buyR1, price: buyPrice } = buyPool;
    const { reserve0: sellR0, reserve1: sellR1, price: sellPrice } = sellPool;
    
    if (sellPrice <= buyPrice) {
        return { size: 0, profit: 0, reason: 'No price advantage' };
    }
    
    // Binary search for optimal trade size
    let low = 0;
    let high = Math.min(buyR1 * 0.1, sellR1 * 0.1); // Max 10% of pool
    let bestSize = 0;
    let bestProfit = -gasUSD; // Start with negative gas cost
    
    while (high - low > 0.01) {
        const size = (low + high) / 2;
        
        // Calculate buy side (USDC -> WMATIC)
        const wmaticToBuy = size / buyPrice;
        const buySlippage = (buyR1 * wmaticToBuy * 997) / (buyR0 * 1000 + wmaticToBuy * 997);
        const actualWmaticReceived = buySlippage;
        
        // Calculate sell side (WMATIC -> USDC)  
        const sellSlippage = (sellR1 * actualWmaticReceived * 997) / (sellR0 * 1000 + actualWmaticReceived * 997);
        const usdcReceived = sellSlippage;
        
        // Calculate profit
        const grossProfit = usdcReceived - size;
        const netProfit = grossProfit - gasUSD;
        
        if (netProfit > bestProfit) {
            bestProfit = netProfit;
            bestSize = size;
            low = size; // Try larger sizes
        } else {
            high = size; // Try smaller sizes
        }
    }
    
    return {
        size: bestSize,
        profit: bestProfit,
        profitable: bestProfit > 0
    };
}

async function findOptimalArbitrage() {
    const pools = [
        { name: 'QuickSwap', pair: '0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827' },
        { name: 'SushiSwap', pair: '0xcd353F79d9FADe311fC3119B841e1f456b54e858' }
    ];
    
    const pairABI = [
        {"constant":true,"inputs":[],"name":"getReserves","outputs":[{"name":"reserve0","type":"uint112"},{"name":"reserve1","type":"uint112"},{"name":"blockTimestampLast","type":"uint32"}],"type":"function"}
    ];
    
    const poolData = [];
    
    for (const pool of pools) {
        const contract = new web3.eth.Contract(pairABI, pool.pair);
        const reserves = await contract.methods.getReserves().call();
        
        const wmatic = Number(reserves.reserve0) / 1e18;
        const usdc = Number(reserves.reserve1) / 1e6;
        const price = usdc / wmatic;
        
        poolData.push({
            name: pool.name,
            reserve0: wmatic,
            reserve1: usdc,
            price: price
        });
    }
    
    // Find best arbitrage direction
    for (let i = 0; i < poolData.length; i++) {
        for (let j = 0; j < poolData.length; j++) {
            if (i === j) continue;
            
            const buyPool = poolData[i];
            const sellPool = poolData[j];
            
            const result = calculateOptimalTradeSize(buyPool, sellPool);
            
            if (result.profitable) {
                console.log(`\n✅ PROFITABLE ARBITRAGE FOUND!`);
                console.log(`   Buy at ${buyPool.name}: $${buyPool.price.toFixed(6)}`);
                console.log(`   Sell at ${sellPool.name}: $${sellPool.price.toFixed(6)}`);
                console.log(`   Optimal Trade Size: $${result.size.toFixed(2)}`);
                console.log(`   Net Profit: $${result.profit.toFixed(2)}`);
                console.log(`   ROI: ${((result.profit / result.size) * 100).toFixed(3)}%`);
            } else {
                const spread = ((sellPool.price - buyPool.price) / buyPool.price) * 100;
                console.log(`\n❌ ${buyPool.name} -> ${sellPool.name}: ${spread.toFixed(3)}% spread`);
                console.log(`   Not profitable after slippage & gas`);
            }
        }
    }
    
    // With Huff gas savings
    console.log('\n=== With Huff Gas Optimization (86% savings) ===');
    const huffGasUSD = 0.50 * 0.14; // 86% reduction
    
    for (let i = 0; i < poolData.length; i++) {
        for (let j = 0; j < poolData.length; j++) {
            if (i === j) continue;
            
            const buyPool = poolData[i];
            const sellPool = poolData[j];
            
            const result = calculateOptimalTradeSize(buyPool, sellPool, huffGasUSD);
            
            if (result.profitable) {
                console.log(`\n✅ PROFITABLE WITH HUFF!`);
                console.log(`   Buy at ${buyPool.name}: $${buyPool.price.toFixed(6)}`);
                console.log(`   Sell at ${sellPool.name}: $${sellPool.price.toFixed(6)}`);
                console.log(`   Optimal Trade Size: $${result.size.toFixed(2)}`);
                console.log(`   Net Profit: $${result.profit.toFixed(2)}`);
                console.log(`   Huff Advantage: $${(0.50 - huffGasUSD).toFixed(2)} extra profit`);
            }
        }
    }
}

findOptimalArbitrage().catch(console.error);