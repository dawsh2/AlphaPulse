const { Web3 } = require('web3');
const web3 = new Web3('https://polygon-rpc.com');

// Check REAL pool prices - NO MOCKS
async function checkRealArbitrage() {
    // These are REAL pool addresses on Polygon
    const pools = [
        { name: 'QuickSwap WMATIC/USDC', pair: '0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827' },
        { name: 'SushiSwap WMATIC/USDC', pair: '0xcd353F79d9FADe311fC3119B841e1f456b54e858' }
    ];
    
    const pairABI = [
        {"constant":true,"inputs":[],"name":"getReserves","outputs":[{"name":"reserve0","type":"uint112"},{"name":"reserve1","type":"uint112"},{"name":"blockTimestampLast","type":"uint32"}],"type":"function"}
    ];
    
    console.log('Fetching REAL pool data from Polygon blockchain...\n');
    
    for (const pool of pools) {
        try {
            const contract = new web3.eth.Contract(pairABI, pool.pair);
            const reserves = await contract.methods.getReserves().call();
            
            // REAL reserves from blockchain
            const wmaticReserve = Number(reserves.reserve0) / 1e18;
            const usdcReserve = Number(reserves.reserve1) / 1e6;
            const price = usdcReserve / wmaticReserve;
            
            console.log(`${pool.name}:`);
            console.log(`  REAL Reserves: ${wmaticReserve.toFixed(2)} WMATIC, ${usdcReserve.toFixed(2)} USDC`);
            console.log(`  Price: 1 WMATIC = $${price.toFixed(6)}`);
            console.log(`  Total Liquidity: $${(usdcReserve * 2).toFixed(2)}`);
            
            // Calculate slippage for $1000 trade
            const tradeUSDC = 1000;
            const wmaticToBuy = tradeUSDC / price;
            
            // Uniswap V2 formula: dy = (y * dx) / (x + dx)
            // With 0.3% fee: dy = (y * dx * 997) / (x * 1000 + dx * 997)
            const dx = wmaticToBuy;
            const x = wmaticReserve;
            const y = usdcReserve;
            
            const dy = (y * dx * 997) / (x * 1000 + dx * 997);
            const effectivePrice = dy / dx;
            const slippage = ((price - effectivePrice) / price) * 100;
            
            console.log(`  Slippage for $${tradeUSDC} trade: ${slippage.toFixed(3)}%`);
            console.log('');
        } catch (err) {
            console.error(`Error fetching ${pool.name}:`, err.message);
        }
    }
    
    console.log('This is REAL data - no mocks, no fakes!');
}

checkRealArbitrage().catch(console.error);