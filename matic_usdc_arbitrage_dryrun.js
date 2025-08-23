const { Web3 } = require('web3');

const web3 = new Web3('https://rpc.ankr.com/polygon/e6fac469b91ea8fd98406aca0820653ae6fe5c2400f44819450f6022dd2792e2');

// Uniswap V2 pair contract ABI
const pairABI = [
  {
    "constant": true,
    "inputs": [],
    "name": "getReserves",
    "outputs": [
      {"name": "_reserve0", "type": "uint112"},
      {"name": "_reserve1", "type": "uint112"},
      {"name": "_blockTimestampLast", "type": "uint32"}
    ],
    "type": "function"
  },
  {
    "constant": true,
    "inputs": [],
    "name": "token0",
    "outputs": [{"name": "", "type": "address"}],
    "type": "function"
  },
  {
    "constant": true,
    "inputs": [],
    "name": "token1", 
    "outputs": [{"name": "", "type": "address"}],
    "type": "function"
  }
];

// ERC20 ABI for getting token info
const erc20ABI = [
  {
    "constant": true,
    "inputs": [],
    "name": "symbol",
    "outputs": [{"name": "", "type": "string"}],
    "type": "function"
  },
  {
    "constant": true,
    "inputs": [],
    "name": "decimals",
    "outputs": [{"name": "", "type": "uint8"}],
    "type": "function"
  }
];

// Calculate Uniswap V2 slippage and price impact
function calculateSlippage(amountIn, reserveIn, reserveOut, decimalsIn, decimalsOut) {
  console.log(`SLIPPAGE DEBUG: amountIn=${amountIn}, reserveIn=${reserveIn}, reserveOut=${reserveOut}`);
  
  const reserveInNum = reserveIn;
  const reserveOutNum = reserveOut;
  
  // Spot price before trade
  const spotPrice = reserveOutNum / reserveInNum;
  console.log(`SLIPPAGE DEBUG: spotPrice=${spotPrice}`);
  
  // Amount after 0.3% fee
  const amountInWithFee = amountIn * 0.997;
  
  // Uniswap V2 constant product formula: (x + Δx)(y - Δy) = xy
  const numerator = amountInWithFee * reserveOutNum;
  const denominator = reserveInNum + amountInWithFee;
  const amountOut = numerator / denominator;
  
  console.log(`SLIPPAGE DEBUG: amountOut=${amountOut}`);
  
  // Effective price per unit
  const effectivePrice = amountOut / amountIn;
  
  // Price impact (how much worse than spot price)
  const priceImpact = ((spotPrice - effectivePrice) / spotPrice) * 100;
  
  console.log(`SLIPPAGE DEBUG: effectivePrice=${effectivePrice}, priceImpact=${priceImpact}`);
  
  return {
    spotPrice,
    effectivePrice,
    amountOut,
    priceImpact: Math.abs(priceImpact),
    slippagePercent: Math.abs(priceImpact)
  };
}

async function getGasPrice() {
  try {
    const gasPrice = await web3.eth.getGasPrice();
    return {
      wei: gasPrice,
      gwei: Number(gasPrice) / 1e9,
      polygonPrice: 0.235 // Approximate POL price in USD
    };
  } catch (error) {
    return {
      wei: '30000000000', // 30 gwei fallback
      gwei: 30,
      polygonPrice: 0.235
    };
  }
}

function calculateGasCosts(gasPriceGwei, polygonPriceUSD) {
  // Typical gas usage for DeFi operations on Polygon
  const swapGasLimit = 150000;    // Single DEX swap
  const approveGasLimit = 50000;  // Token approval
  
  // Two-leg arbitrage: approve + swap + swap
  const totalGasLimit = approveGasLimit + (swapGasLimit * 2);
  
  const gasCostGwei = totalGasLimit * gasPriceGwei;
  const gasCostPOL = gasCostGwei / 1e9;
  const gasCostUSD = gasCostPOL * polygonPriceUSD;
  
  return {
    gasLimit: totalGasLimit,
    costPOL: gasCostPOL,
    costUSD: gasCostUSD
  };
}

async function analyzePool(poolAddress) {
  try {
    const contract = new web3.eth.Contract(pairABI, poolAddress);
    
    const [token0Address, token1Address] = await Promise.all([
      contract.methods.token0().call(),
      contract.methods.token1().call()
    ]);
    
    const token0Contract = new web3.eth.Contract(erc20ABI, token0Address);
    const token1Contract = new web3.eth.Contract(erc20ABI, token1Address);
    
    const [symbol0, symbol1, decimals0, decimals1, reserves] = await Promise.all([
      token0Contract.methods.symbol().call(),
      token1Contract.methods.symbol().call(),
      token0Contract.methods.decimals().call(),
      token1Contract.methods.decimals().call(),
      contract.methods.getReserves().call()
    ]);
    
    const reserve0 = BigInt(reserves._reserve0);
    const reserve1 = BigInt(reserves._reserve1);
    
    const reserve0Human = Number(reserve0) / Math.pow(10, Number(decimals0));
    const reserve1Human = Number(reserve1) / Math.pow(10, Number(decimals1));
    
    // Determine which token is MATIC and which is USDC for consistent pricing
    let maticReserve, usdcReserve, maticIsToken0;
    if (symbol0 === 'MATIC' || symbol0 === 'WMATIC') {
      maticReserve = reserve0Human;
      usdcReserve = reserve1Human;
      maticIsToken0 = true;
    } else {
      maticReserve = reserve1Human;
      usdcReserve = reserve0Human;
      maticIsToken0 = false;
    }
    
    const price = usdcReserve / maticReserve; // USDC per MATIC
    const liquidity = usdcReserve * 2; // Assuming USDC as quote
    
    console.log(`   Token0: ${symbol0} (${Number(decimals0)} decimals) - ${reserve0Human.toFixed(2)}`);
    console.log(`   Token1: ${symbol1} (${Number(decimals1)} decimals) - ${reserve1Human.toFixed(2)}`);
    console.log(`   MATIC Reserve: ${maticReserve.toFixed(2)} | USDC Reserve: ${usdcReserve.toFixed(2)}`);
    
    return {
      address: poolAddress,
      symbol0, symbol1, decimals0, decimals1,
      reserve0, reserve1, reserve0Human, reserve1Human,
      maticReserve, usdcReserve, maticIsToken0,
      price, liquidity,
      executable: liquidity > 5000 && maticReserve > 100 // Lower thresholds for smaller pools
    };
    
  } catch (error) {
    return { address: poolAddress, error: error.message };
  }
}

async function calculateArbitrageProfit(buyPool, sellPool, tradeAmountUSD, gasData) {
  if (!buyPool.executable || !sellPool.executable) {
    return { error: "One or both pools not executable" };
  }
  
  // Convert USD trade amount to MATIC amount (using buy pool price)
  const tradeAmountTokens = tradeAmountUSD / buyPool.price;
  
  console.log(`\nDEBUG: Trade ${tradeAmountUSD} = ${tradeAmountTokens.toFixed(0)} MATIC at ${buyPool.price.toFixed(6)}`);
  
  // For buying: We trade USDC for MATIC
  let buySlippage;
  if (buyPool.maticIsToken0) {
    // USDC -> MATIC (token1 -> token0)
    buySlippage = calculateSlippage(
      tradeAmountUSD, buyPool.reserve1Human, buyPool.reserve0Human, 
      6, 18  // USDC has 6 decimals, MATIC has 18
    );
  } else {
    // USDC -> MATIC (token0 -> token1)
    buySlippage = calculateSlippage(
      tradeAmountUSD, buyPool.reserve0Human, buyPool.reserve1Human,
      6, 18  // USDC has 6 decimals, MATIC has 18
    );
  }
  
  // Amount of MATIC we get after buying
  const tokensReceived = buySlippage.amountOut;
  
  console.log(`DEBUG: Buying gives us ${tokensReceived.toFixed(0)} MATIC (${buySlippage.slippagePercent.toFixed(3)}% slippage)`);
  
  // For selling: We trade MATIC for USDC
  let sellSlippage;
  if (sellPool.maticIsToken0) {
    // MATIC -> USDC (token0 -> token1)
    sellSlippage = calculateSlippage(
      tokensReceived, sellPool.reserve0Human, sellPool.reserve1Human,
      18, 6  // MATIC has 18 decimals, USDC has 6
    );
  } else {
    // MATIC -> USDC (token1 -> token0)
    sellSlippage = calculateSlippage(
      tokensReceived, sellPool.reserve1Human, sellPool.reserve0Human,
      18, 6  // MATIC has 18 decimals, USDC has 6
    );
  }
  
  // Final USDC amount after selling
  const finalUSDCAmount = sellSlippage.amountOut;
  
  console.log(`DEBUG: Selling gives us ${finalUSDCAmount.toFixed(2)} USDC (${sellSlippage.slippagePercent.toFixed(3)}% slippage)`);
  
  // Calculate costs
  const gasCosts = calculateGasCosts(gasData.gwei, gasData.polygonPrice);
  const dexFees = tradeAmountUSD * 0.006; // 0.3% each direction
  const totalCosts = gasCosts.costUSD + dexFees;
  
  // Calculate profit
  const grossProfit = finalUSDCAmount - tradeAmountUSD;
  const netProfit = grossProfit - totalCosts;
  const netProfitPercent = (netProfit / tradeAmountUSD) * 100;
  
  console.log(`DEBUG: Gross profit: ${grossProfit.toFixed(3)}, Costs: ${totalCosts.toFixed(3)}, Net: ${netProfit.toFixed(3)}`);
  
  return {
    tradeAmountUSD,
    tradeAmountTokens,
    buyPrice: buyPool.price,
    sellPrice: sellPool.price,
    tokensReceived,
    finalUSDCAmount,
    grossProfit,
    costs: {
      gas: gasCosts.costUSD,
      dexFees,
      total: totalCosts
    },
    buySlippage: buySlippage.slippagePercent,
    sellSlippage: sellSlippage.slippagePercent,
    netProfit,
    netProfitPercent,
    profitable: netProfit > 0
  };
}

async function main() {
  console.log('🚀 MATIC/USDC ARBITRAGE DRY RUN ANALYSIS');
  console.log('🎯 Based on detected opportunity: 1.405% spread');
  console.log('📊 LIQUIDITY: $50,000 | GAS COST: $0.001 | NET PROFIT: 1.031% ($35.13)');
  console.log('🎯 SAFE SIZE: $2,500.00');
  console.log('⚠️  NOTE: Lower liquidity - expect higher slippage but larger spread!\n');
  
  // Pool addresses from the opportunity with expected prices
  const pools = [
    { address: '0xcd353f79d9fade311fc3119b841e1f456b54e858', price: 0.230576, size: 50000 },
    { address: '0x380615f37993b5a96adf3d443b6e0ac50a211998', price: 0.233817, size: 25000 }
  ];
  
  console.log('📊 Getting current gas prices...');
  const gasData = await getGasPrice();
  console.log(`Gas Price: ${gasData.gwei.toFixed(1)} gwei`);
  console.log(`POL Price: $${gasData.polygonPrice}`);
  
  console.log('\n🔍 Analyzing pools...');
  const poolData = [];
  
  for (let i = 0; i < pools.length; i++) {
    console.log(`\n=== POOL ${i+1}/${pools.length} ===`);
    const result = await analyzePool(pools[i].address);
    if (!result.error && result.executable) {
      poolData.push(result);
      console.log(`✅ Pool ${i+1}: ${result.symbol0}/${result.symbol1}`);
      console.log(`   Price: $${result.price.toFixed(6)} (Expected: $${pools[i].price.toFixed(6)})`);
      console.log(`   Liquidity: $${result.liquidity.toFixed(0)}`);
      console.log(`   Price Deviation: ${(((result.price - pools[i].price) / pools[i].price) * 100).toFixed(3)}%`);
    } else {
      console.log(`❌ Pool ${i+1}: ${result.error || 'Not executable'}`);
      if (result.symbol0 && result.symbol1) {
        console.log(`   Detected: ${result.symbol0}/${result.symbol1}`);
        console.log(`   Liquidity: $${result.liquidity?.toFixed(0) || 'Unknown'}`);
        console.log(`   Reason: ${!result.executable ? 'Insufficient liquidity/reserves' : result.error}`);
      }
    }
    console.log('─'.repeat(60));
  }
  
  if (poolData.length < 2) {
    console.log('\n❌ Not enough executable pools for arbitrage');
    if (poolData.length === 1) {
      console.log('💡 Only one pool is executable - cannot perform arbitrage');
      console.log('🔧 Check if second pool has sufficient liquidity or different token symbols');
    }
    return;
  }
  
  // Sort by price to find cheapest and most expensive
  poolData.sort((a, b) => a.price - b.price);
  const cheapestPool = poolData[0];
  const mostExpensivePool = poolData[poolData.length - 1];
  
  console.log(`\n🎯 ARBITRAGE OPPORTUNITY DETECTED:`);
  console.log(`📈 Buy at:  $${cheapestPool.price.toFixed(6)} (Pool: ${cheapestPool.address.slice(0,10)}...)`);
  console.log(`📉 Sell at: $${mostExpensivePool.price.toFixed(6)} (Pool: ${mostExpensivePool.address.slice(0,10)}...)`);
  
  const spotSpread = ((mostExpensivePool.price - cheapestPool.price) / cheapestPool.price) * 100;
  console.log(`📊 Spot Spread: ${spotSpread.toFixed(3)}%`);
  
  // Test smaller trade sizes due to limited liquidity
  const tradeSizes = [500, 1000, 1500, 2000, 2500, 3000, 4000, 5000];
  
  console.log(`\n💸 PROFIT ANALYSIS BY TRADE SIZE:`);
  console.log(`┌──────────┬────────────┬──────────┬──────────────┬───────────────┬─────────────┐`);
  console.log(`│ Trade $  │ Net Profit │ Profit % │ Buy Slippage │ Sell Slippage │ Profitable? │`);
  console.log(`├──────────┼────────────┼──────────┼──────────────┼───────────────┼─────────────┤`);
  
  let optimalSize = 0;
  let maxProfit = -Infinity;
  let maxProfitPercent = -Infinity;
  let results = [];
  
  for (const tradeSize of tradeSizes) {
    const analysis = await calculateArbitrageProfit(
      cheapestPool, mostExpensivePool, tradeSize, gasData
    );
    
    if (analysis.error) {
      console.log(`│ $${tradeSize.toString().padEnd(7)} │ ERROR: ${analysis.error.slice(0,35).padEnd(35)} │`);
      continue;
    }
    
    results.push(analysis);
    
    const profitable = analysis.profitable ? '✅ YES' : '❌ NO';
    const profit = analysis.netProfit.toFixed(2);
    const profitPercent = analysis.netProfitPercent.toFixed(3);
    const buySlip = analysis.buySlippage.toFixed(3);
    const sellSlip = analysis.sellSlippage.toFixed(3);
    
    console.log(`│ $${tradeSize.toString().padEnd(7)} │ $${profit.padEnd(9)} │ ${profitPercent.padEnd(7)}% │ ${buySlip.padEnd(11)}% │ ${sellSlip.padEnd(12)}% │ ${profitable.padEnd(10)} │`);
    
    if (analysis.netProfit > maxProfit) {
      maxProfit = analysis.netProfit;
      maxProfitPercent = analysis.netProfitPercent;
      optimalSize = tradeSize;
    }
  }
  
  console.log(`└──────────┴────────────┴──────────┴──────────────┴───────────────┴─────────────┘`);
  
  // Detailed breakdown for optimal trade size
  const detailedAnalysis = optimalSize > 0 ? 
    results.find(r => r.tradeAmountUSD === optimalSize) : 
    results[Math.floor(results.length / 2)]; // Use middle result if none profitable
  
  if (detailedAnalysis) {
    console.log(`\n📋 DETAILED BREAKDOWN (${optimalSize > 0 ? 'OPTIMAL' : 'EXAMPLE'}: $${detailedAnalysis.tradeAmountUSD} trade):`);
    console.log(`┌─────────────────────────────────────────────────────────────┐`);
    console.log(`│ 🔄 ARBITRAGE EXECUTION PLAN                                 │`);
    console.log(`├─────────────────────────────────────────────────────────────┤`);
    console.log(`│ 1️⃣  Input: $${detailedAnalysis.tradeAmountUSD.toFixed(0)} USDC                                       │`);
    console.log(`│ 2️⃣  Buy ${detailedAnalysis.tradeAmountTokens.toFixed(0)} MATIC at $${detailedAnalysis.buyPrice.toFixed(6)}              │`);
    console.log(`│     → Receive ${detailedAnalysis.tokensReceived.toFixed(0)} MATIC (${detailedAnalysis.buySlippage.toFixed(3)}% slippage)     │`);
    console.log(`│ 3️⃣  Sell ${detailedAnalysis.tokensReceived.toFixed(0)} MATIC at $${detailedAnalysis.sellPrice.toFixed(6)}              │`);
    console.log(`│     → Receive $${detailedAnalysis.finalUSDCAmount.toFixed(2)} USDC (${detailedAnalysis.sellSlippage.toFixed(3)}% slippage)        │`);
    console.log(`├─────────────────────────────────────────────────────────────┤`);
    console.log(`│ 💰 COST BREAKDOWN                                           │`);
    console.log(`├─────────────────────────────────────────────────────────────┤`);
    console.log(`│ Gas Cost: $${detailedAnalysis.costs.gas.toFixed(4)} (${gasData.gwei.toFixed(1)} gwei)                              │`);
    console.log(`│ DEX Fees: $${detailedAnalysis.costs.dexFees.toFixed(2)} (0.3% × 2)                           │`);
    console.log(`│ Total Costs: $${detailedAnalysis.costs.total.toFixed(2)}                                    │`);
    console.log(`├─────────────────────────────────────────────────────────────┤`);
    console.log(`│ 📊 PROFIT SUMMARY                                           │`);
    console.log(`├─────────────────────────────────────────────────────────────┤`);
    console.log(`│ Gross Profit: $${detailedAnalysis.grossProfit.toFixed(3)}                                  │`);
    console.log(`│ Net Profit: $${detailedAnalysis.netProfit.toFixed(3)} (${detailedAnalysis.netProfitPercent.toFixed(3)}%)                          │`);
    console.log(`│ Status: ${detailedAnalysis.profitable ? '✅ PROFITABLE' : '❌ UNPROFITABLE'}                               │`);
    console.log(`└─────────────────────────────────────────────────────────────┘`);
  }
  
  console.log(`\n🎯 FINAL RECOMMENDATION:`);
  if (spotSpread > 1.0 && maxProfit > 10) {
    console.log(`🔥 EXCELLENT OPPORTUNITY: ${spotSpread.toFixed(3)}% spread, $${maxProfit.toFixed(2)} max profit at $${optimalSize}`);
    console.log(`⚡ Action: EXECUTE with $${optimalSize} for optimal risk/reward`);
    console.log(`🚨 Risk Level: MEDIUM (lower liquidity but high spread)`);
    console.log(`⏰ Speed Priority: HIGH - larger spreads disappear quickly`);
  } else if (spotSpread > 0.8 && maxProfit > 5) {
    console.log(`✅ GOOD OPPORTUNITY: ${spotSpread.toFixed(3)}% spread, $${maxProfit.toFixed(2)} max profit at $${optimalSize}`);
    console.log(`⚡ Action: Consider executing with $${optimalSize || 2500}`);
    console.log(`🚨 Risk Level: MEDIUM-HIGH (manage slippage carefully)`);
  } else if (spotSpread > 0.5) {
    console.log(`⚠️ MARGINAL: ${spotSpread.toFixed(3)}% spread, high slippage impact`);
    console.log(`💡 Consider smaller position or wait for better liquidity`);
  } else {
    console.log(`❌ SKIP: Only ${spotSpread.toFixed(3)}% spread, or execution issues`);
    console.log(`⏰ Wait for better opportunity or check pool accessibility`);
  }
  
  console.log(`\n📈 MARKET CONDITIONS:`);
  console.log(`🔥 Gas Price: ${gasData.gwei.toFixed(1)} gwei (${gasData.gwei < 50 ? 'GOOD' : gasData.gwei < 100 ? 'OK' : 'HIGH'})`);
  console.log(`💧 Total Liquidity: $${poolData.reduce((sum, p) => sum + p.liquidity, 0).toFixed(0)}`);
  console.log(`🎯 Pool Count: ${poolData.length}/2 pools executable`);
  console.log(`📊 Average Pool Size: $${poolData.length > 0 ? (poolData.reduce((sum, p) => sum + p.liquidity, 0) / poolData.length).toFixed(0) : 'N/A'}`);
  console.log(`⏱️ Expected Execution Time: <30 seconds (if both pools accessible)`);
  console.log(`🎯 Success Probability: ${detailedAnalysis?.profitable ? 'HIGH (80-90%)' : 'MEDIUM (50-70%)'}`);
  
  // Compare with expected dashboard values
  console.log(`\n📊 DASHBOARD vs REALITY CHECK:`);
  console.log(`Expected Spread: 1.405% | Actual Spread: ${spotSpread.toFixed(3)}%`);
  console.log(`Expected Profit: $35.13 | Actual Max Profit: $${maxProfit.toFixed(2)}`);
  console.log(`Expected Safe Size: $2,500 | Optimal Size: $${optimalSize || 'N/A'}`);
  
  let accuracy;
  if (maxProfit > 25 && spotSpread > 1.0) {
    accuracy = 'EXCELLENT';
  } else if (maxProfit > 10 && spotSpread > 0.5) {
    accuracy = 'GOOD';
  } else if (spotSpread > 0.3) {
    accuracy = 'PARTIAL';
  } else {
    accuracy = 'POOR';
  }
  console.log(`Dashboard Accuracy: ${accuracy}`);
  
  // Risk assessment for low liquidity
  console.log(`\n⚠️  LOW LIQUIDITY RISK ASSESSMENT:`);
  console.log(`• Slippage Impact: ${results.length > 0 ? 'HIGH' : 'UNKNOWN'} - Monitor execution carefully`);
  console.log(`• Front-running Risk: MEDIUM - Smaller pools harder to sandwich`);
  console.log(`• Execution Speed: CRITICAL - Large spreads attract competition`);
  console.log(`• Position Sizing: Use smaller amounts than high-liquidity opportunities`);
}

// Error handling
process.on('unhandledRejection', (reason, promise) => {
  console.error('Unhandled Rejection at:', promise, 'reason:', reason);
});

process.on('uncaughtException', (error) => {
  console.error('Uncaught Exception:', error);
  process.exit(1);
});

// Run the analysis
main().catch(console.error);
