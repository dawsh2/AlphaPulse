const { Web3 } = require('web3');
const web3 = new Web3('https://polygon-rpc.com');

// Test that we're using REAL pool data and closed-form optimal sizing
async function testRealArbitrage() {
    console.log('Testing real arbitrage with closed-form optimal sizing...\n');
    
    // Real pool addresses on Polygon mainnet
    const pools = {
        quickswap: '0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827', // WMATIC/USDC
        sushiswap: '0xcd353F79d9FADe311fC3119B841e1f456b54e858'  // WMATIC/USDC
    };
    
    // Uniswap V2 Pair ABI (minimal)
    const pairABI = [
        {
            "constant": true,
            "inputs": [],
            "name": "getReserves",
            "outputs": [
                {"name": "reserve0", "type": "uint112"},
                {"name": "reserve1", "type": "uint112"},
                {"name": "blockTimestampLast", "type": "uint32"}
            ],
            "stateMutability": "view",
            "type": "function"
        }
    ];
    
    // Fetch REAL reserves from blockchain
    const quickswapPair = new web3.eth.Contract(pairABI, pools.quickswap);
    const sushiswapPair = new web3.eth.Contract(pairABI, pools.sushiswap);
    
    const [quickReserves, sushiReserves] = await Promise.all([
        quickswapPair.methods.getReserves().call(),
        sushiswapPair.methods.getReserves().call()
    ]);
    
    // Calculate prices (USDC has 6 decimals, WMATIC has 18)
    const quickPrice = (Number(quickReserves.reserve1) / 1e6) / (Number(quickReserves.reserve0) / 1e18); // USD per WMATIC
    const sushiPrice = (Number(sushiReserves.reserve1) / 1e6) / (Number(sushiReserves.reserve0) / 1e18);
    
    console.log('REAL Pool Data:');
    console.log('QuickSwap WMATIC/USDC:');
    console.log(`  Reserve0 (WMATIC): ${Number(quickReserves.reserve0) / 1e18}`);
    console.log(`  Reserve1 (USDC): ${Number(quickReserves.reserve1) / 1e6}`);
    console.log(`  Price: $${quickPrice.toFixed(6)}\n`);
    
    console.log('SushiSwap WMATIC/USDC:');
    console.log(`  Reserve0 (WMATIC): ${Number(sushiReserves.reserve0) / 1e18}`);
    console.log(`  Reserve1 (USDC): ${Number(sushiReserves.reserve1) / 1e6}`);
    console.log(`  Price: $${sushiPrice.toFixed(6)}\n`);
    
    // Calculate optimal trade size using CLOSED-FORM solution
    const priceDiff = Math.abs(quickPrice - sushiPrice);
    const avgPrice = (quickPrice + sushiPrice) / 2;
    const spreadPct = (priceDiff / avgPrice) * 100;
    
    console.log('Arbitrage Analysis:');
    console.log(`  Price spread: ${spreadPct.toFixed(4)}%`);
    
    if (spreadPct < 0.1) {
        console.log('  ❌ Spread too small for profitable arbitrage\n');
        return;
    }
    
    // Closed-form optimal trade size calculation
    // For Uniswap V2: optimal = sqrt(r_in * r_out * r'_out * r'_in * f1 * f2) - r_in * f1
    //                           ---------------------------------------------------
    //                                                f1
    const fee = 0.997; // 0.3% fee
    
    let optimalAmount;
    if (quickPrice < sushiPrice) {
        // Buy from QuickSwap, sell to SushiSwap
        const r1_in = Number(quickReserves.reserve0);
        const r1_out = Number(quickReserves.reserve1);
        const r2_in = Number(sushiReserves.reserve1);
        const r2_out = Number(sushiReserves.reserve0);
        
        const sqrtArg = r1_in * r1_out * r2_out * r2_in * fee * fee;
        optimalAmount = (Math.sqrt(sqrtArg) - r1_in * fee) / fee / 1e18; // Convert to WMATIC
    } else {
        // Buy from SushiSwap, sell to QuickSwap
        const r1_in = Number(sushiReserves.reserve0);
        const r1_out = Number(sushiReserves.reserve1);
        const r2_in = Number(quickReserves.reserve1);
        const r2_out = Number(quickReserves.reserve0);
        
        const sqrtArg = r1_in * r1_out * r2_out * r2_in * fee * fee;
        optimalAmount = (Math.sqrt(sqrtArg) - r1_in * fee) / fee / 1e18; // Convert to WMATIC
    }
    
    // Calculate profit at optimal size
    const optimalUSD = optimalAmount * avgPrice;
    const gasUSD = 0.50; // Estimate with Huff optimization
    
    // Calculate slippage at optimal size
    const slippagePct = (optimalAmount * 1e18) / 
        (quickPrice < sushiPrice ? Number(quickReserves.reserve0) : Number(sushiReserves.reserve0)) * 100;
    
    const grossProfit = optimalUSD * (spreadPct / 100);
    const netProfit = grossProfit - gasUSD;
    
    console.log('\nClosed-Form Optimal Trade Size:');
    console.log(`  Optimal amount: ${optimalAmount.toFixed(4)} WMATIC`);
    console.log(`  Optimal USD value: $${optimalUSD.toFixed(2)}`);
    console.log(`  Expected slippage: ${slippagePct.toFixed(3)}%`);
    console.log(`  Gross profit: $${grossProfit.toFixed(2)}`);
    console.log(`  Gas cost: $${gasUSD.toFixed(2)}`);
    console.log(`  Net profit: $${netProfit.toFixed(2)}`);
    
    if (netProfit > 0) {
        console.log('  ✅ Profitable arbitrage opportunity!');
    } else {
        console.log('  ❌ Not profitable after gas costs');
    }
}

testRealArbitrage().catch(console.error);