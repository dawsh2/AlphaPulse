// Gas Profiling Framework for Solidity Baseline Establishment
// This script establishes comprehensive gas usage baselines for comparison with Huff implementations

const { ethers } = require("ethers");
const fs = require("fs");
const path = require("path");

class GasProfiler {
    constructor(config) {
        this.provider = new ethers.providers.JsonRpcProvider(config.rpcUrl);
        this.wallet = new ethers.Wallet(config.privateKey, this.provider);
        this.config = config;
        this.results = {
            timestamp: new Date().toISOString(),
            network: config.network,
            gasPrice: null,
            scenarios: {},
            summary: {}
        };
    }

    async initialize() {
        // Get current gas price
        const gasPrice = await this.provider.getGasPrice();
        this.results.gasPrice = gasPrice.toString();
        
        console.log(`🔧 Initialized gas profiler on ${this.config.network}`);
        console.log(`⛽ Current gas price: ${ethers.utils.formatUnits(gasPrice, "gwei")} gwei`);
    }

    async profileSolidityBaseline(contractAddress, testSuiteAddress) {
        console.log("📊 Starting Solidity baseline profiling...");
        
        // Load contract ABIs
        const optimizedContract = await this.loadContract(contractAddress, "FlashArbitrageOptimized");
        const testSuite = await this.loadContract(testSuiteAddress, "ArbitrageTestSuite");

        // Profile individual scenarios
        await this.profileScenario("simple_arbitrage", optimizedContract, {
            flashAmount: ethers.utils.parseUnits("1000", 6), // 1000 USDC
            minProfit: ethers.utils.parseUnits("1", 6)
        });

        await this.profileScenario("triangular_arbitrage", optimizedContract, {
            flashAmount: ethers.utils.parseUnits("500", 6),
            minProfit: ethers.utils.parseUnits("0.5", 6)
        });

        await this.profileScenario("small_trade", optimizedContract, {
            flashAmount: ethers.utils.parseUnits("50", 6),
            minProfit: ethers.utils.parseUnits("0.01", 6)
        });

        await this.profileScenario("large_trade", optimizedContract, {
            flashAmount: ethers.utils.parseUnits("10000", 6),
            minProfit: ethers.utils.parseUnits("10", 6)
        });

        // Profile comprehensive test suite
        await this.profileTestSuite(testSuite);

        // Generate detailed analysis
        this.generateGasAnalysis();
        
        // Save baseline results
        await this.saveResults();
        
        console.log("✅ Solidity baseline profiling complete");
        return this.results;
    }

    async profileScenario(scenarioName, contract, params) {
        console.log(`🧪 Profiling scenario: ${scenarioName}`);
        
        const iterations = 5; // Multiple runs for average
        const gasUsages = [];
        let successful = 0;
        
        for (let i = 0; i < iterations; i++) {
            try {
                // Estimate gas first
                const gasEstimate = await contract.estimateGas.executeArbitrage(
                    params.flashAmount,
                    params.minProfit
                );

                // Execute transaction with gas measurement
                const tx = await contract.executeArbitrage(
                    params.flashAmount,
                    params.minProfit,
                    {
                        gasLimit: gasEstimate.mul(110).div(100) // 10% buffer
                    }
                );

                const receipt = await tx.wait();
                gasUsages.push(receipt.gasUsed.toNumber());
                successful++;

                console.log(`  Run ${i + 1}: ${receipt.gasUsed.toNumber()} gas`);
                
                // Small delay between runs
                await new Promise(resolve => setTimeout(resolve, 1000));
                
            } catch (error) {
                console.log(`  Run ${i + 1}: Failed - ${error.message}`);
                // Record failed attempts for analysis
                gasUsages.push(0);
            }
        }

        // Calculate statistics
        const validGasUsages = gasUsages.filter(gas => gas > 0);
        const stats = this.calculateGasStatistics(validGasUsages);
        
        this.results.scenarios[scenarioName] = {
            parameters: params,
            iterations,
            successful,
            failureRate: ((iterations - successful) / iterations) * 100,
            gasStatistics: stats,
            rawData: gasUsages
        };

        console.log(`  📈 ${scenarioName}: avg=${stats.average}, min=${stats.min}, max=${stats.max}`);
    }

    async profileTestSuite(testSuite) {
        console.log("🧪 Profiling comprehensive test suite...");
        
        try {
            const gasEstimate = await testSuite.estimateGas.runFullTestSuite();
            
            const tx = await testSuite.runFullTestSuite({
                gasLimit: gasEstimate.mul(120).div(100) // 20% buffer for comprehensive tests
            });

            const receipt = await tx.wait();
            
            // Get detailed test results
            const testResults = await testSuite.getTestResults();
            
            this.results.scenarios.comprehensive_suite = {
                totalGasUsed: receipt.gasUsed.toNumber(),
                testCount: testResults.length,
                testResults: testResults.map(result => ({
                    testName: result.testName,
                    gasUsed: result.gasUsed.toNumber(),
                    success: result.success,
                    profit: result.profit.toString()
                }))
            };

            console.log(`  📊 Comprehensive suite: ${receipt.gasUsed.toNumber()} total gas`);
            
        } catch (error) {
            console.log(`  ❌ Test suite failed: ${error.message}`);
            this.results.scenarios.comprehensive_suite = {
                error: error.message,
                failed: true
            };
        }
    }

    calculateGasStatistics(gasUsages) {
        if (gasUsages.length === 0) {
            return { average: 0, min: 0, max: 0, median: 0, stdDev: 0 };
        }

        const sorted = gasUsages.sort((a, b) => a - b);
        const sum = gasUsages.reduce((acc, val) => acc + val, 0);
        const average = Math.round(sum / gasUsages.length);
        const min = sorted[0];
        const max = sorted[sorted.length - 1];
        const median = sorted[Math.floor(sorted.length / 2)];
        
        // Calculate standard deviation
        const variance = gasUsages.reduce((acc, val) => acc + Math.pow(val - average, 2), 0) / gasUsages.length;
        const stdDev = Math.round(Math.sqrt(variance));

        return { average, min, max, median, stdDev };
    }

    generateGasAnalysis() {
        console.log("📈 Generating gas analysis...");
        
        const scenarios = Object.entries(this.results.scenarios);
        const validScenarios = scenarios.filter(([_, data]) => !data.failed && data.gasStatistics);
        
        if (validScenarios.length === 0) {
            console.log("⚠️  No valid scenarios for analysis");
            return;
        }

        // Calculate overall statistics
        const allGasUsages = validScenarios.flatMap(([_, data]) => 
            data.gasStatistics ? [data.gasStatistics.average] : []
        );

        const overallStats = this.calculateGasStatistics(allGasUsages);
        
        // Calculate gas costs in USD (approximate)
        const maticPriceUsd = 0.80; // Approximate MATIC price
        const avgGasCostMatic = (overallStats.average * parseInt(this.results.gasPrice)) / 1e18;
        const avgGasCostUsd = avgGasCostMatic * maticPriceUsd;

        this.results.summary = {
            totalScenarios: scenarios.length,
            successfulScenarios: validScenarios.length,
            overallGasStatistics: overallStats,
            estimatedCosts: {
                averageGasCostMatic: avgGasCostMatic.toFixed(6),
                averageGasCostUsd: avgGasCostUsd.toFixed(4),
                maticPriceUsed: maticPriceUsd
            },
            optimizationTargets: this.identifyOptimizationTargets(validScenarios)
        };

        console.log(`📊 Analysis complete:`);
        console.log(`  • Average gas usage: ${overallStats.average}`);
        console.log(`  • Average cost: $${avgGasCostUsd.toFixed(4)} USD`);
        console.log(`  • Successful scenarios: ${validScenarios.length}/${scenarios.length}`);
    }

    identifyOptimizationTargets(validScenarios) {
        // Identify scenarios with highest gas usage for optimization priority
        const priorityTargets = validScenarios
            .map(([name, data]) => ({
                scenario: name,
                avgGas: data.gasStatistics.average,
                maxGas: data.gasStatistics.max,
                variability: data.gasStatistics.stdDev
            }))
            .sort((a, b) => b.avgGas - a.avgGas)
            .slice(0, 3);

        return {
            highestGasUsage: priorityTargets,
            highVariability: validScenarios
                .filter(([_, data]) => data.gasStatistics.stdDev > data.gasStatistics.average * 0.1)
                .map(([name, data]) => ({
                    scenario: name,
                    stdDev: data.gasStatistics.stdDev,
                    coefficient: (data.gasStatistics.stdDev / data.gasStatistics.average * 100).toFixed(2)
                }))
        };
    }

    async loadContract(address, contractName) {
        // In a real implementation, this would load the ABI from artifacts
        // For now, we'll create a minimal interface
        const abi = this.getContractABI(contractName);
        return new ethers.Contract(address, abi, this.wallet);
    }

    getContractABI(contractName) {
        // Minimal ABIs for testing - in production these would be loaded from artifacts
        const abis = {
            FlashArbitrageOptimized: [
                "function executeArbitrage(uint256 flashAmount, uint256 minProfit) external",
                "function getExecutionStats() external view returns (uint256, uint256, uint256, uint256)",
                "function checkProfitability(uint256 flashAmount) external view returns (uint256, uint256, uint256, uint256, bool)"
            ],
            ArbitrageTestSuite: [
                "function runFullTestSuite() external returns (uint256, uint256)",
                "function getTestResults() external view returns (tuple(string testName, uint256 gasUsed, bool success, uint256 profit, uint256 timestamp)[])",
                "function getGasBaseline(string memory testName) external view returns (uint256)"
            ]
        };
        
        return abis[contractName] || [];
    }

    async saveResults() {
        const filename = `baseline_gas_${Date.now()}.json`;
        const filepath = path.join(__dirname, "../monitoring", filename);
        
        // Ensure monitoring directory exists
        const monitoringDir = path.dirname(filepath);
        if (!fs.existsSync(monitoringDir)) {
            fs.mkdirSync(monitoringDir, { recursive: true });
        }
        
        fs.writeFileSync(filepath, JSON.stringify(this.results, null, 2));
        
        // Also save as latest baseline
        const latestPath = path.join(__dirname, "../monitoring", "baseline_gas_latest.json");
        fs.writeFileSync(latestPath, JSON.stringify(this.results, null, 2));
        
        console.log(`💾 Results saved to ${filepath}`);
        
        // Generate human-readable report
        await this.generateReport();
    }

    async generateReport() {
        const reportPath = path.join(__dirname, "../monitoring", "gas_profiling_report.md");
        
        let report = `# Gas Profiling Report\n\n`;
        report += `**Generated:** ${this.results.timestamp}\n`;
        report += `**Network:** ${this.config.network}\n`;
        report += `**Gas Price:** ${ethers.utils.formatUnits(this.results.gasPrice, "gwei")} gwei\n\n`;
        
        report += `## Summary\n\n`;
        if (this.results.summary) {
            report += `- **Average Gas Usage:** ${this.results.summary.overallGasStatistics.average}\n`;
            report += `- **Average Cost:** $${this.results.summary.estimatedCosts.averageGasCostUsd} USD\n`;
            report += `- **Successful Scenarios:** ${this.results.summary.successfulScenarios}/${this.results.summary.totalScenarios}\n\n`;
        }
        
        report += `## Scenario Results\n\n`;
        for (const [scenario, data] of Object.entries(this.results.scenarios)) {
            if (data.failed) {
                report += `### ${scenario} ❌\n`;
                report += `**Status:** Failed - ${data.error}\n\n`;
            } else if (data.gasStatistics) {
                report += `### ${scenario} ✅\n`;
                report += `- **Average Gas:** ${data.gasStatistics.average}\n`;
                report += `- **Min/Max:** ${data.gasStatistics.min} / ${data.gasStatistics.max}\n`;
                report += `- **Standard Deviation:** ${data.gasStatistics.stdDev}\n`;
                report += `- **Success Rate:** ${((data.successful / data.iterations) * 100).toFixed(1)}%\n\n`;
            }
        }
        
        report += `## Optimization Targets\n\n`;
        if (this.results.summary && this.results.summary.optimizationTargets) {
            report += `### Highest Gas Usage\n`;
            for (const target of this.results.summary.optimizationTargets.highestGasUsage) {
                report += `- **${target.scenario}:** ${target.avgGas} gas average\n`;
            }
        }
        
        fs.writeFileSync(reportPath, report);
        console.log(`📄 Report generated: ${reportPath}`);
    }
}

// Configuration
const config = {
    network: process.env.NETWORK || "polygon-mumbai",
    rpcUrl: process.env.RPC_URL || "https://rpc-mumbai.maticvigil.com",
    privateKey: process.env.PRIVATE_KEY || "your-private-key-here"
};

// Export for use in other scripts
module.exports = { GasProfiler };

// CLI usage
if (require.main === module) {
    async function main() {
        const contractAddress = process.argv[2];
        const testSuiteAddress = process.argv[3];
        
        if (!contractAddress || !testSuiteAddress) {
            console.log("Usage: node gas_profiler.js <contract-address> <test-suite-address>");
            process.exit(1);
        }
        
        const profiler = new GasProfiler(config);
        await profiler.initialize();
        await profiler.profileSolidityBaseline(contractAddress, testSuiteAddress);
    }
    
    main().catch(console.error);
}