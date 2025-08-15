// Capital-Based Arbitrage Bot (Using Your Own Capital)
// Simple arbitrage using your existing balances - no flash loans needed
const { ethers } = require('ethers');
const WebSocket = require('ws');

// DEX Router ABIs (simplified)
const UNISWAP_V2_ROUTER_ABI = [
    "function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)",
    "function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)"
];

const ERC20_ABI = [
    "function balanceOf(address owner) view returns (uint256)",
    "function approve(address spender, uint256 amount) returns (bool)",
    "function transfer(address to, uint256 amount) returns (bool)"
];

// Polygon DEX Routers
const DEX_ROUTERS = {
    quickswap: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff",
    sushiswap: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"
};

const POLYGON_TOKENS = {
    WMATIC: "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270",
    USDC: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
    USDT: "0xc2132D05D31c914a87C6611C10748AEb04B58e8F",
    WETH: "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619"
};

class CapitalArbBot {
    constructor(config) {
        this.provider = new ethers.providers.JsonRpcProvider(config.rpcUrl);
        this.wallet = new ethers.Wallet(config.privateKey, this.provider);
        
        // No smart contract needed - just direct DEX interactions
        this.routers = {};
        Object.entries(DEX_ROUTERS).forEach(([name, address]) => {
            this.routers[name] = new ethers.Contract(address, UNISWAP_V2_ROUTER_ABI, this.wallet);
        });
        
        // Token contracts for approvals
        this.tokens = {};
        Object.entries(POLYGON_TOKENS).forEach(([symbol, address]) => {
            this.tokens[symbol] = new ethers.Contract(address, ERC20_ABI, this.wallet);
        });
        
        this.config = config;
        this.scannerWS = null;
        this.executionQueue = [];
        this.isExecuting = false;
    }

    async start() {
        console.log('🚀 Starting Capital-Based Arbitrage Bot');
        console.log('💰 Using your own capital - no flash loans required');
        console.log('💡 Simple two-step arbitrage for testing');
        
        // Check wallet balances
        await this.checkBalances();
        
        // Connect to scanner
        await this.connectToScanner();
        
        // Start execution loop
        this.startExecutionLoop();
        
        console.log('✅ Bot running - ready for opportunities');
    }

    async checkBalances() {
        console.log('\n💳 Wallet Balances:');
        
        for (const [symbol, contract] of Object.entries(this.tokens)) {
            try {
                const balance = await contract.balanceOf(this.wallet.address);
                const decimals = symbol === 'USDC' || symbol === 'USDT' ? 6 : 18;
                const formatted = ethers.utils.formatUnits(balance, decimals);
                console.log(`   ${symbol}: ${formatted}`);
                
                if (parseFloat(formatted) === 0) {
                    console.log(`   ⚠️  No ${symbol} balance - can't arbitrage with this token`);
                }
            } catch (error) {
                console.log(`   ❌ Error checking ${symbol} balance`);
            }
        }
        console.log('');
    }

    async connectToScanner() {
        this.scannerWS = new WebSocket('ws://localhost:8080/opportunities');
        
        this.scannerWS.on('message', (data) => {
            const opportunity = JSON.parse(data);
            this.handleOpportunity(opportunity);
        });

        this.scannerWS.on('error', (error) => {
            console.error('Scanner connection error:', error);
        });
    }

    handleOpportunity(opportunity) {
        console.log('📊 New opportunity:', opportunity.pair, `$${opportunity.estimatedProfit}`);
        
        if (this.isValidOpportunity(opportunity)) {
            this.executionQueue.push({
                ...opportunity,
                timestamp: Date.now(),
                attempts: 0
            });
            console.log(`✅ Queued: ${opportunity.pair} - $${opportunity.estimatedProfit}`);
        } else {
            console.log(`❌ Rejected: Below threshold or insufficient balance`);
        }
    }

    async isValidOpportunity(opp) {
        // Check if we have enough balance for this trade
        const tokenSymbol = this.getTokenSymbol(opp.tokenA);
        if (!tokenSymbol) return false;
        
        const balance = await this.tokens[tokenSymbol].balanceOf(this.wallet.address);
        const decimals = tokenSymbol === 'USDC' || tokenSymbol === 'USDT' ? 6 : 18;
        const balanceFormatted = parseFloat(ethers.utils.formatUnits(balance, decimals));
        
        // Calculate required trade size (conservative)
        const requiredBalance = opp.estimatedProfit / opp.profitPercent; // Approximate trade size
        
        return (
            opp.estimatedProfit >= this.config.minProfitUSD &&
            opp.profitPercent > 0.002 && // 0.2% minimum (higher than flash loans due to simpler execution)
            balanceFormatted >= requiredBalance &&
            Date.now() - opp.detectedAt < 5000
        );
    }

    getTokenSymbol(address) {
        for (const [symbol, tokenAddress] of Object.entries(POLYGON_TOKENS)) {
            if (tokenAddress.toLowerCase() === address.toLowerCase()) {
                return symbol;
            }
        }
        return null;
    }

    startExecutionLoop() {
        setInterval(async () => {
            if (this.executionQueue.length > 0 && !this.isExecuting) {
                const opportunity = this.executionQueue.shift();
                await this.executeArbitrage(opportunity);
            }
        }, 100);
    }

    async executeArbitrage(opportunity) {
        this.isExecuting = true;
        
        try {
            console.log(`⚡ EXECUTING: ${opportunity.pair}`);
            console.log(`   Expected profit: $${opportunity.estimatedProfit}`);
            
            // Calculate trade size based on available balance
            const tradeSize = await this.calculateTradeSize(opportunity);
            
            if (tradeSize === 0) {
                console.log('❌ Insufficient balance for trade');
                return;
            }
            
            // Execute capital-based arbitrage (two transactions in quick succession)
            const profit = await this.executeTwoStepSwaps(opportunity, tradeSize);
            
            if (profit > 0) {
                console.log(`✅ SUCCESS: Made $${profit.toFixed(2)} profit`);
            } else {
                console.log(`❌ LOSS: Lost $${Math.abs(profit).toFixed(2)}`);
            }
            
        } catch (error) {
            console.error('❌ Execution failed:', error.message);
        } finally {
            this.isExecuting = false;
        }
    }

    async calculateTradeSize(opportunity) {
        const tokenSymbol = this.getTokenSymbol(opportunity.tokenA);
        const balance = await this.tokens[tokenSymbol].balanceOf(this.wallet.address);
        const decimals = tokenSymbol === 'USDC' || tokenSymbol === 'USDT' ? 6 : 18;
        const balanceFormatted = parseFloat(ethers.utils.formatUnits(balance, decimals));
        
        // Use up to 50% of available balance for safety
        const maxTradeSize = balanceFormatted * 0.5;
        
        // Calculate optimal size based on liquidity
        const liquidityConstraint = Math.min(opportunity.liquidityA, opportunity.liquidityB) * 0.05;
        
        const optimalSize = Math.min(maxTradeSize, liquidityConstraint);
        
        console.log(`   Trade size: ${optimalSize.toFixed(2)} ${tokenSymbol}`);
        return ethers.utils.parseUnits(optimalSize.toString(), decimals);
    }

    async executeTwoStepSwaps(opportunity, tradeSize) {
        const startTime = Date.now();
        
        // Get router contracts
        const routerBuy = this.routers[opportunity.dexA];
        const routerSell = this.routers[opportunity.dexB];
        
        if (!routerBuy || !routerSell) {
            throw new Error('Unknown DEX routers');
        }
        
        const tokenIn = opportunity.tokenA;
        const tokenOut = opportunity.tokenB;
        
        console.log(`   Step 1: Buy ${this.getTokenSymbol(tokenOut)} on ${opportunity.dexA}`);
        
        // Approve tokenIn for first DEX
        const tokenInContract = new ethers.Contract(tokenIn, ERC20_ABI, this.wallet);
        const approveTx1 = await tokenInContract.approve(routerBuy.address, tradeSize);
        await approveTx1.wait();
        
        // Execute first swap (buy)
        const path1 = [tokenIn, tokenOut];
        const deadline = Math.floor(Date.now() / 1000) + 300;
        
        const swapTx1 = await routerBuy.swapExactTokensForTokens(
            tradeSize,
            0, // Accept any amount (risky but simpler)
            path1,
            this.wallet.address,
            deadline,
            {
                gasLimit: 300000,
                gasPrice: await this.getOptimalGasPrice()
            }
        );
        
        const receipt1 = await swapTx1.wait();
        console.log(`   ✅ Buy completed: ${receipt1.transactionHash}`);
        
        // Get amount received from first swap
        const tokenOutContract = new ethers.Contract(tokenOut, ERC20_ABI, this.wallet);
        const tokenOutBalance = await tokenOutContract.balanceOf(this.wallet.address);
        
        console.log(`   Step 2: Sell ${this.getTokenSymbol(tokenOut)} on ${opportunity.dexB}`);
        
        // Approve tokenOut for second DEX
        const approveTx2 = await tokenOutContract.approve(routerSell.address, tokenOutBalance);
        await approveTx2.wait();
        
        // Execute second swap (sell)
        const path2 = [tokenOut, tokenIn];
        
        const swapTx2 = await routerSell.swapExactTokensForTokens(
            tokenOutBalance,
            0,
            path2,
            this.wallet.address,
            deadline,
            {
                gasLimit: 300000,
                gasPrice: await this.getOptimalGasPrice()
            }
        );
        
        const receipt2 = await swapTx2.wait();
        console.log(`   ✅ Sell completed: ${receipt2.transactionHash}`);
        
        // Calculate profit
        const endBalance = await tokenInContract.balanceOf(this.wallet.address);
        const profit = endBalance.sub(tradeSize);
        
        const decimals = this.getTokenSymbol(tokenIn) === 'USDC' || this.getTokenSymbol(tokenIn) === 'USDT' ? 6 : 18;
        const profitFormatted = parseFloat(ethers.utils.formatUnits(profit, decimals));
        
        const executionTime = Date.now() - startTime;
        console.log(`   ⏱️  Execution time: ${executionTime}ms`);
        
        return profitFormatted;
    }

    async getOptimalGasPrice() {
        const currentGas = await this.provider.getGasPrice();
        return currentGas.mul(110).div(100); // 10% premium
    }
}

// Configuration for capital-based arbitrage
const config = {
    rpcUrl: 'https://polygon-mainnet.public.blastapi.io',
    privateKey: process.env.PRIVATE_KEY,
    minProfitUSD: 5, // Lower threshold since we're not paying flash loan fees
    maxGasPrice: 100
};

// Start the bot
const bot = new CapitalArbBot(config);
bot.start().catch(console.error);

// Example: Manual arbitrage execution
async function manualArbitrage() {
    const opportunity = {
        pair: "WMATIC-USDC",
        tokenA: POLYGON_TOKENS.USDC,
        tokenB: POLYGON_TOKENS.WMATIC,
        dexA: "quickswap",
        dexB: "sushiswap",
        estimatedProfit: 25,
        profitPercent: 0.003,
        liquidityA: 100000,
        liquidityB: 80000,
        detectedAt: Date.now()
    };
    
    await bot.executeArbitrage(opportunity);
}

// Uncomment to test manual execution
// manualArbitrage().catch(console.error);
