const Web3 = require('web3');
const web3 = new Web3('https://polygon-rpc.com');

// Check real pool prices
async function checkArbitrage() {
    // WMATIC/USDC pools on different DEXs
    const pools = [
        { name: 'QuickSwap', pair: '0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827' },
        { name: 'SushiSwap', pair: '0xcd353F79d9FADe311fC3119B841e1f456b54e858' }
    ];
    
    const pairABI = [
        {"constant":true,"inputs":[],"name":"getReserves","outputs":[{"name":"reserve0","type":"uint112"},{"name":"reserve1","type":"uint112"},{"name":"blockTimestampLast","type":"uint32"}],"type":"function"},
        {"constant":true,"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},
        {"constant":true,"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}
    ];
    
    console.log('Checking real DEX pool prices on Polygon...\n');
    
    const prices = [];
    for (const pool of pools) {
        const contract = new web3.eth.Contract(pairABI, pool.pair);
        const reserves = await contract.methods.getReserves().call();
        const price = parseFloat(reserves.reserve1) / parseFloat(reserves.reserve0) * 1e12; // USDC per WMATIC (adjusting decimals)
        prices.push({ name: pool.name, price, reserves });
        
        console.log(`${pool.name}: 1 WMATIC = $${price.toFixed(6)}`);
        console.log(`  Reserves: ${(parseFloat(reserves.reserve0)/1e18).toFixed(2)} WMATIC, ${(parseFloat(reserves.reserve1)/1e6).toFixed(2)} USDC`);
    }
    
    // Check for arbitrage
    if (prices.length > 1) {
        const minPrice = Math.min(...prices.map(p => p.price));
        const maxPrice = Math.max(...prices.map(p => p.price));
        const spread = ((maxPrice - minPrice) / minPrice) * 100;
        
        console.log(`\n=== Arbitrage Analysis ===`);
        console.log(`Price spread: ${spread.toFixed(4)}%`);
        console.log(`Buy at: $${minPrice.toFixed(6)}`);
        console.log(`Sell at: $${maxPrice.toFixed(6)}`);
        
        // Calculate actual profit with slippage
        const tradeSize = 1000; // $1000 trade
        const wmaticAmount = tradeSize / minPrice;
        
        // Estimate slippage (simplified)
        const buyPool = prices.find(p => p.price === minPrice);
        const sellPool = prices.find(p => p.price === maxPrice);
        
        const buyReserve0 = parseFloat(buyPool.reserves.reserve0) / 1e18;
        const buyReserve1 = parseFloat(buyPool.reserves.reserve1) / 1e6;
        
        // Uniswap formula with 0.3% fee
        const amountInWithFee = wmaticAmount * 0.997;
        const numerator = amountInWithFee * buyReserve1;
        const denominator = buyReserve0 + amountInWithFee;
        const amountOut = numerator / denominator;
        
        const actualBuyPrice = tradeSize / amountOut;
        const slippage = ((actualBuyPrice - minPrice) / minPrice) * 100;
        
        console.log(`\nWith $${tradeSize} trade:`);
        console.log(`  Slippage: ${slippage.toFixed(2)}%`);
        console.log(`  Actual profit: $${((maxPrice - actualBuyPrice) * wmaticAmount - 0.5).toFixed(2)} (after ~$0.50 gas)`);
        
        if (spread > 0.1) {
            console.log(`\n⚠️ WARNING: ${spread.toFixed(2)}% spread detected!`);
            if (spread > 1) {
                console.log(`This is likely stale data or different token pairs.`);
            }
        }
    }
}

checkArbitrage().catch(console.error);