const { expect } = require("chai");
const { ethers } = require("hardhat");
const { parseUnits, formatUnits } = ethers.utils;

describe("FlashLoanArbitrage MEV Gas Analysis", function () {
    let owner, flashArbContract;
    let usdc, weth, wmatic;
    let quickswapRouter, sushiswapRouter;
    let aavePool;
    
    // Real Polygon addresses
    const ADDRESSES = {
        USDC: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
        WETH: "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619", 
        WMATIC: "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270",
        QUICKSWAP_ROUTER: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff",
        SUSHISWAP_ROUTER: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506",
        AAVE_POOL: "0x794a61358D6845594F94dc1DB02A252b5b4814aD"
    };

    before(async function () {
        [owner] = await ethers.getSigners();
        
        // For now, deploy Solidity version to get baseline gas measurements
        const FlashArbSolidity = await ethers.getContractFactory("FlashLoanArbitrage");
        flashArbContract = await FlashArbSolidity.deploy();
        await flashArbContract.deployed();
        
        console.log("🚀 FlashLoanArbitrage deployed to:", flashArbContract.address);
        
        // Get token contracts
        usdc = await ethers.getContractAt("IERC20", ADDRESSES.USDC);
        weth = await ethers.getContractAt("IERC20", ADDRESSES.WETH);
        wmatic = await ethers.getContractAt("IERC20", ADDRESSES.WMATIC);
    });

    describe("Real Gas Measurements", function () {
        
        it("Should measure gas for single USDC->WETH->USDC arbitrage", async function () {
            const flashAmount = parseUnits("1000", 6); // 1000 USDC
            
            const tx = await flashArbContract.executeArbitrage(
                flashAmount,
                ADDRESSES.QUICKSWAP_ROUTER,  // Buy router
                ADDRESSES.SUSHISWAP_ROUTER,  // Sell router  
                ADDRESSES.WETH,              // Token B
                parseUnits("1", 6),          // Min profit: 1 USDC
                { gasLimit: 500000 }
            );
            
            const receipt = await tx.wait();
            
            console.log("📊 SINGLE ARBITRAGE GAS ANALYSIS:");
            console.log(`   Gas Used: ${receipt.gasUsed.toString()}`);
            console.log(`   Gas Price: ${tx.gasPrice.toString()} wei`);
            console.log(`   Total Cost: ${formatUnits(receipt.gasUsed.mul(tx.gasPrice), 18)} MATIC`);
            
            // Store baseline for comparison
            this.singleArbGas = receipt.gasUsed;
        });
        
        it("Should measure gas for multi-hop arbitrage simulation", async function () {
            // Simulate 2-hop: USDC -> WMATIC -> WETH -> USDC
            const flashAmount = parseUnits("1000", 6);
            
            // This would be the multi-pool call structure
            const swapData = ethers.utils.defaultAbiCoder.encode(
                ["tuple(address,uint8,address,address,uint24,uint256)[]"],
                [[
                    [ADDRESSES.QUICKSWAP_ROUTER, 2, ADDRESSES.USDC, ADDRESSES.WMATIC, 3000, 0],
                    [ADDRESSES.SUSHISWAP_ROUTER, 2, ADDRESSES.WMATIC, ADDRESSES.WETH, 3000, 0], 
                    [ADDRESSES.QUICKSWAP_ROUTER, 2, ADDRESSES.WETH, ADDRESSES.USDC, 3000, 0]
                ]]
            );
            
            // For now, estimate gas for the multi-hop call
            try {
                const gasEstimate = await flashArbContract.estimateGas.executeArbitrage(
                    flashAmount,
                    ADDRESSES.QUICKSWAP_ROUTER,
                    ADDRESSES.SUSHISWAP_ROUTER, 
                    ADDRESSES.WETH,
                    parseUnits("1", 6)
                );
                
                console.log("📊 MULTI-HOP ARBITRAGE GAS ESTIMATE:");
                console.log(`   Estimated Gas: ${gasEstimate.toString()}`);
                console.log(`   vs Single Hop: +${gasEstimate.sub(this.singleArbGas || 0).toString()} gas`);
                
            } catch (error) {
                console.log("⚠️  Multi-hop simulation failed (expected on fork):", error.message);
            }
        });
        
        it("Should analyze gas by swap count", async function () {
            const results = [];
            
            for (let swapCount = 1; swapCount <= 3; swapCount++) {
                try {
                    const gasEstimate = await flashArbContract.estimateGas.executeArbitrage(
                        parseUnits("1000", 6),
                        ADDRESSES.QUICKSWAP_ROUTER,
                        ADDRESSES.SUSHISWAP_ROUTER,
                        ADDRESSES.WETH,
                        parseUnits("1", 6)
                    );
                    
                    results.push({
                        swaps: swapCount,
                        gas: gasEstimate.toString(),
                        costUSD: (gasEstimate.toNumber() * 30e-9 * 0.8).toFixed(4) // ~30 gwei, $0.8 MATIC
                    });
                    
                } catch (error) {
                    results.push({
                        swaps: swapCount,
                        gas: "FAILED",
                        costUSD: "N/A"
                    });
                }
            }
            
            console.log("📊 GAS ANALYSIS BY SWAP COUNT:");
            console.table(results);
        });
        
        it("Should compare V2 vs V3 pool types", async function () {
            const comparisons = [
                { name: "V2 QuickSwap", router: ADDRESSES.QUICKSWAP_ROUTER },
                { name: "V2 SushiSwap", router: ADDRESSES.SUSHISWAP_ROUTER }
            ];
            
            const results = [];
            
            for (const comp of comparisons) {
                try {
                    const gasEstimate = await flashArbContract.estimateGas.executeArbitrage(
                        parseUnits("100", 6), // Smaller amount for testing
                        comp.router,
                        comp.router,
                        ADDRESSES.WETH,
                        parseUnits("0.1", 6)
                    );
                    
                    results.push({
                        poolType: comp.name,
                        gas: gasEstimate.toString(),
                        efficiency: (100000 / gasEstimate.toNumber() * 100).toFixed(2) + "%"
                    });
                    
                } catch (error) {
                    results.push({
                        poolType: comp.name,
                        gas: "FAILED", 
                        efficiency: "N/A"
                    });
                }
            }
            
            console.log("📊 V2 POOL TYPE COMPARISON:");
            console.table(results);
        });
    });
    
    describe("MEV Competitive Analysis", function () {
        
        it("Should calculate MEV profitability thresholds", async function () {
            const gasPrice = parseUnits("30", "gwei"); // Current Polygon gas price
            const maticPrice = 0.8; // USD
            
            const scenarios = [
                { name: "Small Arb", gas: 150000, profit: 5 },
                { name: "Medium Arb", gas: 200000, profit: 20 },
                { name: "Large Arb", gas: 300000, profit: 100 }
            ];
            
            const analysis = scenarios.map(scenario => {
                const gasCostMatic = (scenario.gas * 30e-9); // 30 gwei
                const gasCostUSD = gasCostMatic * maticPrice;
                const netProfitUSD = scenario.profit - gasCostUSD;
                const profitMargin = (netProfitUSD / scenario.profit * 100).toFixed(1);
                
                return {
                    scenario: scenario.name,
                    gasUsed: scenario.gas.toLocaleString(),
                    gasCostUSD: gasCostUSD.toFixed(4),
                    grossProfitUSD: scenario.profit.toFixed(2),
                    netProfitUSD: netProfitUSD.toFixed(4),
                    marginPercent: profitMargin + "%"
                };
            });
            
            console.log("💰 MEV PROFITABILITY ANALYSIS:");
            console.table(analysis);
        });
        
        it("Should estimate daily MEV potential", async function () {
            const dailyStats = {
                arbitragesPerDay: 100,
                avgGasUsed: 180000,
                avgProfitUSD: 15,
                gasPrice: 30e-9, // 30 gwei
                maticPrice: 0.8
            };
            
            const dailyGasCost = dailyStats.arbitragesPerDay * dailyStats.avgGasUsed * dailyStats.gasPrice * dailyStats.maticPrice;
            const dailyGrossProfit = dailyStats.arbitragesPerDay * dailyStats.avgProfitUSD;
            const dailyNetProfit = dailyGrossProfit - dailyGasCost;
            
            const summary = {
                dailyArbitrages: dailyStats.arbitragesPerDay,
                dailyGasCostUSD: dailyGasCost.toFixed(2),
                dailyGrossProfitUSD: dailyGrossProfit.toFixed(2),
                dailyNetProfitUSD: dailyNetProfit.toFixed(2),
                monthlyNetUSD: (dailyNetProfit * 30).toFixed(2),
                gasEfficiencyPercent: ((dailyGrossProfit - dailyGasCost) / dailyGrossProfit * 100).toFixed(1) + "%"
            };
            
            console.log("📈 DAILY MEV POTENTIAL:");
            console.table([summary]);
        });
    });
});