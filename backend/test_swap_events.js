const { Web3 } = require('web3');
const web3 = new Web3('https://polygon-rpc.com');

// Monitor real DEX swaps and show what data we get
async function monitorSwaps() {
    console.log('Monitoring Real DEX Swaps on Polygon...\n');
    
    // QuickSwap WMATIC/USDC pool
    const poolAddress = '0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827';
    
    // Swap event signature
    const swapTopic = '0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822';
    
    // Get recent swaps
    const latestBlock = await web3.eth.getBlockNumber();
    const fromBlock = latestBlock - 10n; // Last 10 blocks (smaller range for public RPC)
    
    const logs = await web3.eth.getPastLogs({
        address: poolAddress,
        topics: [swapTopic],
        fromBlock: fromBlock,
        toBlock: 'latest'
    });
    
    console.log(`Found ${logs.length} swaps in last 100 blocks\n`);
    
    // Parse swap events
    for (const log of logs.slice(0, 3)) { // Show first 3
        // Decode swap data
        const data = log.data;
        const amount0In = BigInt('0x' + data.slice(2, 66));
        const amount1In = BigInt('0x' + data.slice(66, 130));
        const amount0Out = BigInt('0x' + data.slice(130, 194));
        const amount1Out = BigInt('0x' + data.slice(194, 258));
        
        console.log('Real Swap Event:');
        console.log('  Pool:', poolAddress);
        console.log('  Block:', log.blockNumber);
        console.log('  TX:', log.transactionHash);
        console.log('  Data:');
        console.log(`    WMATIC In: ${Number(amount0In) / 1e18}`);
        console.log(`    USDC In: ${Number(amount1In) / 1e6}`);
        console.log(`    WMATIC Out: ${Number(amount0Out) / 1e18}`);
        console.log(`    USDC Out: ${Number(amount1Out) / 1e6}`);
        
        // Calculate effective price
        if (amount0In > 0n && amount1Out > 0n) {
            const price = Number(amount1Out) / 1e6 / (Number(amount0In) / 1e18);
            console.log(`  Price: $${price.toFixed(6)} per WMATIC`);
        } else if (amount1In > 0n && amount0Out > 0n) {
            const price = Number(amount1In) / 1e6 / (Number(amount0Out) / 1e18);
            console.log(`  Price: $${price.toFixed(6)} per WMATIC`);
        }
        
        console.log('\nThis is REAL data our scanner needs!');
        console.log('- Pool address (for identifying DEX)');
        console.log('- Swap amounts (for calculating price/volume)');
        console.log('- Block number (for ordering)');
        console.log('- NO fake pools or estimated reserves!\n');
        console.log('---');
    }
    
    // Now fetch current pool reserves for context
    const pairABI = [{
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
    }];
    
    const pair = new web3.eth.Contract(pairABI, poolAddress);
    const reserves = await pair.methods.getReserves().call();
    
    console.log('\nCurrent Pool Reserves:');
    console.log(`  WMATIC: ${Number(reserves.reserve0) / 1e18}`);
    console.log(`  USDC: ${Number(reserves.reserve1) / 1e6}`);
    console.log(`  Spot Price: $${(Number(reserves.reserve1) / 1e6) / (Number(reserves.reserve0) / 1e18)}`);
    
    console.log('\n✅ With SwapEventMessages, our scanner gets:');
    console.log('  - Real swap data from actual DEX transactions');
    console.log('  - Pool addresses to identify arbitrage paths');
    console.log('  - Accurate amounts for slippage calculation');
    console.log('  - Block numbers for proper ordering');
    console.log('\n❌ Without them (using TradeMessages):');
    console.log('  - Fake pools created from individual trades');
    console.log('  - Made-up reserves (volume * 100)');
    console.log('  - No real liquidity data');
    console.log('  - False arbitrage opportunities');
}

monitorSwaps().catch(console.error);