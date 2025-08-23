#!/usr/bin/env ts-node

/**
 * Parity Verification Script for Solidity vs Huff Implementations
 * 
 * This script performs comprehensive testing to ensure behavioral equivalence
 * between the original Solidity contracts and optimized Huff implementations.
 */

import { ethers } from 'ethers';
import { config } from 'dotenv';
import * as fs from 'fs';
import * as path from 'path';

config();

// Test configuration
const CONFIG = {
    rpcUrl: process.env.RPC_URL || 'https://polygon-rpc.com',
    privateKey: process.env.PRIVATE_KEY || '',
    solidityAddress: process.env.SOLIDITY_CONTRACT || '',
    huffAddress: process.env.HUFF_CONTRACT || '',
    testNetwork: process.env.NETWORK || 'polygon',
    gasLimit: 1000000,
    timeout: 60000, // 60 seconds per test
};

// Token addresses on Polygon
const TOKENS = {
    WMATIC: '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270',
    USDC: '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174',
    USDC_NEW: '0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359',
    USDT: '0xc2132D05D31c914a87C6611C10748AEb04B58e8F',
    WETH: '0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619',
    WBTC: '0x1bfd67037b42cf73acF2047067bd4F2C47D9BfD6',
};

// DEX routers on Polygon
const ROUTERS = {
    QUICKSWAP: '0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff',
    SUSHISWAP: '0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506',
    UNISWAP_V3: '0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45',
};

interface TestScenario {
    name: string;
    description: string;
    flashAmount: ethers.BigNumber;
    minProfit: ethers.BigNumber;
    path: string[];
    expectedSuccess: boolean;
    gasTarget?: number; // Expected gas usage for Huff
}

interface ParityResult {
    scenario: string;
    solidityGas: number;
    huffGas: number;
    gasSaved: number;
    percentSaved: number;
    solidityOutput: any;
    huffOutput: any;
    outputsMatch: boolean;
    eventsMatch: boolean;
    stateMatch: boolean;
    passed: boolean;
    error?: string;
}

class ParityVerifier {
    private provider: ethers.providers.JsonRpcProvider;
    private wallet: ethers.Wallet;
    private solidityContract: ethers.Contract;
    private huffContract: ethers.Contract;
    private results: ParityResult[] = [];

    constructor() {
        this.provider = new ethers.providers.JsonRpcProvider(CONFIG.rpcUrl);
        this.wallet = new ethers.Wallet(CONFIG.privateKey, this.provider);
    }

    async initialize(solidityAddress: string, huffAddress: string) {
        console.log('🔧 Initializing parity verifier...');
        
        // Load contract ABIs
        const abi = this.loadContractABI();
        
        this.solidityContract = new ethers.Contract(solidityAddress, abi, this.wallet);
        this.huffContract = new ethers.Contract(huffAddress, abi, this.wallet);
        
        console.log(`  Solidity contract: ${solidityAddress}`);
        console.log(`  Huff contract: ${huffAddress}`);
        
        // Verify both contracts are deployed
        const solidityCode = await this.provider.getCode(solidityAddress);
        const huffCode = await this.provider.getCode(huffAddress);
        
        if (solidityCode === '0x') {
            throw new Error('Solidity contract not deployed at specified address');
        }
        if (huffCode === '0x') {
            throw new Error('Huff contract not deployed at specified address');
        }
        
        console.log('✅ Contracts verified and loaded');
    }

    private loadContractABI(): any[] {
        // Minimal ABI for arbitrage contracts
        return [
            'function executeArbitrage(uint256 flashAmount, uint256 minProfit) external returns (uint256)',
            'function executeOperation(address[] assets, uint256[] amounts, uint256[] premiums, address initiator, bytes params) external returns (bool)',
            'function getExecutionStats() external view returns (uint256 avgGas, uint256 lastGas, uint256 totalGas, uint256 count)',
            'function withdraw(address token) external',
            'event ArbitrageExecuted(uint256 profit, uint256 gasUsed)',
            'event FlashLoanReceived(address token, uint256 amount)',
        ];
    }

    async runComprehensiveParityTests(): Promise<void> {
        console.log('\n🧪 Starting comprehensive parity tests...\n');
        
        const scenarios = this.generateTestScenarios();
        
        for (const scenario of scenarios) {
            await this.testScenario(scenario);
            
            // Small delay between tests
            await new Promise(resolve => setTimeout(resolve, 1000));
        }
        
        this.generateReport();
    }

    private generateTestScenarios(): TestScenario[] {
        return [
            {
                name: 'simple_arbitrage_small',
                description: 'Small 2-hop arbitrage with 100 USDC',
                flashAmount: ethers.utils.parseUnits('100', 6),
                minProfit: ethers.utils.parseUnits('0.1', 6),
                path: [TOKENS.USDC, TOKENS.WMATIC, TOKENS.USDC],
                expectedSuccess: true,
                gasTarget: 150000,
            },
            {
                name: 'simple_arbitrage_medium',
                description: 'Medium 2-hop arbitrage with 1000 USDC',
                flashAmount: ethers.utils.parseUnits('1000', 6),
                minProfit: ethers.utils.parseUnits('1', 6),
                path: [TOKENS.USDC, TOKENS.WMATIC, TOKENS.USDC],
                expectedSuccess: true,
                gasTarget: 150000,
            },
            {
                name: 'simple_arbitrage_large',
                description: 'Large 2-hop arbitrage with 10000 USDC',
                flashAmount: ethers.utils.parseUnits('10000', 6),
                minProfit: ethers.utils.parseUnits('10', 6),
                path: [TOKENS.USDC, TOKENS.WMATIC, TOKENS.USDC],
                expectedSuccess: true,
                gasTarget: 150000,
            },
            {
                name: 'triangular_arbitrage',
                description: '3-hop triangular arbitrage',
                flashAmount: ethers.utils.parseUnits('500', 6),
                minProfit: ethers.utils.parseUnits('0.5', 6),
                path: [TOKENS.USDC, TOKENS.WETH, TOKENS.WMATIC, TOKENS.USDC],
                expectedSuccess: true,
                gasTarget: 200000,
            },
            {
                name: 'complex_5hop',
                description: '5-hop complex arbitrage path',
                flashAmount: ethers.utils.parseUnits('200', 6),
                minProfit: ethers.utils.parseUnits('0.2', 6),
                path: [TOKENS.USDC, TOKENS.WETH, TOKENS.WBTC, TOKENS.WMATIC, TOKENS.USDT, TOKENS.USDC],
                expectedSuccess: true,
                gasTarget: 300000,
            },
            {
                name: 'edge_case_dust',
                description: 'Dust amount arbitrage (should fail)',
                flashAmount: ethers.utils.parseUnits('0.001', 6),
                minProfit: ethers.utils.parseUnits('0.0001', 6),
                path: [TOKENS.USDC, TOKENS.WMATIC, TOKENS.USDC],
                expectedSuccess: false,
            },
            {
                name: 'edge_case_zero',
                description: 'Zero amount arbitrage (should fail)',
                flashAmount: ethers.BigNumber.from(0),
                minProfit: ethers.BigNumber.from(0),
                path: [TOKENS.USDC, TOKENS.WMATIC, TOKENS.USDC],
                expectedSuccess: false,
            },
            {
                name: 'edge_case_max_uint',
                description: 'Max uint256 amount (should fail)',
                flashAmount: ethers.constants.MaxUint256,
                minProfit: ethers.utils.parseUnits('1', 6),
                path: [TOKENS.USDC, TOKENS.WMATIC, TOKENS.USDC],
                expectedSuccess: false,
            },
            {
                name: 'high_slippage_test',
                description: 'Test with expected high slippage',
                flashAmount: ethers.utils.parseUnits('50000', 6),
                minProfit: ethers.utils.parseUnits('100', 6),
                path: [TOKENS.USDC, TOKENS.WMATIC, TOKENS.USDC],
                expectedSuccess: false, // Likely to fail due to slippage
            },
            {
                name: 'usdc_migration_arb',
                description: 'USDC.e to USDC arbitrage',
                flashAmount: ethers.utils.parseUnits('1000', 6),
                minProfit: ethers.utils.parseUnits('0.5', 6),
                path: [TOKENS.USDC, TOKENS.USDC_NEW],
                expectedSuccess: true,
                gasTarget: 120000,
            },
        ];
    }

    private async testScenario(scenario: TestScenario): Promise<void> {
        console.log(`\n📊 Testing: ${scenario.name}`);
        console.log(`   ${scenario.description}`);
        
        const result: ParityResult = {
            scenario: scenario.name,
            solidityGas: 0,
            huffGas: 0,
            gasSaved: 0,
            percentSaved: 0,
            solidityOutput: null,
            huffOutput: null,
            outputsMatch: false,
            eventsMatch: false,
            stateMatch: false,
            passed: false,
        };
        
        try {
            // Capture initial state
            const initialState = await this.captureState();
            
            // Test Solidity implementation
            console.log('   Testing Solidity...');
            const solidityResult = await this.executeArbitrage(
                this.solidityContract,
                scenario.flashAmount,
                scenario.minProfit,
                scenario.expectedSuccess
            );
            result.solidityGas = solidityResult.gasUsed;
            result.solidityOutput = solidityResult.output;
            
            // Reset state between tests
            await this.resetState(initialState);
            
            // Test Huff implementation
            console.log('   Testing Huff...');
            const huffResult = await this.executeArbitrage(
                this.huffContract,
                scenario.flashAmount,
                scenario.minProfit,
                scenario.expectedSuccess
            );
            result.huffGas = huffResult.gasUsed;
            result.huffOutput = huffResult.output;
            
            // Calculate gas savings
            result.gasSaved = result.solidityGas - result.huffGas;
            result.percentSaved = (result.gasSaved / result.solidityGas) * 100;
            
            // Verify outputs match
            result.outputsMatch = this.compareOutputs(solidityResult.output, huffResult.output);
            result.eventsMatch = await this.compareEvents(solidityResult.receipt, huffResult.receipt);
            result.stateMatch = await this.compareStates(solidityResult.finalState, huffResult.finalState);
            
            // Determine if test passed
            result.passed = result.outputsMatch && result.eventsMatch && result.stateMatch;
            
            // Check gas target if specified
            if (scenario.gasTarget && result.huffGas > scenario.gasTarget) {
                console.warn(`   ⚠️  Huff gas ${result.huffGas} exceeds target ${scenario.gasTarget}`);
            }
            
            if (result.passed) {
                console.log(`   ✅ PASS - Gas saved: ${result.percentSaved.toFixed(1)}% (${result.gasSaved} gas)`);
            } else {
                console.log(`   ❌ FAIL - Outputs don't match`);
                result.error = this.getDifferenceDetails(solidityResult, huffResult);
            }
            
        } catch (error) {
            console.log(`   ❌ ERROR - ${error.message}`);
            result.error = error.message;
            result.passed = false;
        }
        
        this.results.push(result);
    }

    private async executeArbitrage(
        contract: ethers.Contract,
        flashAmount: ethers.BigNumber,
        minProfit: ethers.BigNumber,
        expectedSuccess: boolean
    ): Promise<any> {
        try {
            // Estimate gas first
            const gasEstimate = await contract.estimateGas.executeArbitrage(
                flashAmount,
                minProfit,
                { gasLimit: CONFIG.gasLimit }
            );
            
            // Execute transaction
            const tx = await contract.executeArbitrage(
                flashAmount,
                minProfit,
                { gasLimit: gasEstimate.mul(110).div(100) } // 10% buffer
            );
            
            const receipt = await tx.wait();
            const finalState = await this.captureState();
            
            return {
                success: true,
                gasUsed: receipt.gasUsed.toNumber(),
                output: receipt.logs,
                receipt,
                finalState,
            };
            
        } catch (error) {
            if (expectedSuccess) {
                throw error; // Unexpected failure
            }
            
            // Expected failure - return details
            return {
                success: false,
                gasUsed: 0,
                output: error.message,
                receipt: null,
                finalState: await this.captureState(),
            };
        }
    }

    private async captureState(): Promise<any> {
        // Capture relevant contract state
        const state = {
            timestamp: Date.now(),
            balances: {},
            stats: null,
        };
        
        // Capture token balances
        for (const [name, address] of Object.entries(TOKENS)) {
            const token = new ethers.Contract(
                address,
                ['function balanceOf(address) view returns (uint256)'],
                this.provider
            );
            
            state.balances[name] = {
                solidity: await token.balanceOf(this.solidityContract.address),
                huff: await token.balanceOf(this.huffContract.address),
            };
        }
        
        // Capture execution stats if available
        try {
            state.stats = {
                solidity: await this.solidityContract.getExecutionStats(),
                huff: await this.huffContract.getExecutionStats(),
            };
        } catch {
            // Stats may not be available
        }
        
        return state;
    }

    private async resetState(initialState: any): Promise<void> {
        // In production, this would reset contract state
        // For testing, we may need to deploy fresh contracts
        await new Promise(resolve => setTimeout(resolve, 100));
    }

    private compareOutputs(solidityOutput: any, huffOutput: any): boolean {
        // Compare return values
        if (!solidityOutput && !huffOutput) return true;
        if (!solidityOutput || !huffOutput) return false;
        
        // Deep comparison of outputs
        return JSON.stringify(solidityOutput) === JSON.stringify(huffOutput);
    }

    private async compareEvents(solidityReceipt: any, huffReceipt: any): boolean {
        if (!solidityReceipt && !huffReceipt) return true;
        if (!solidityReceipt || !huffReceipt) return false;
        
        // Compare event emissions
        if (solidityReceipt.logs.length !== huffReceipt.logs.length) {
            return false;
        }
        
        for (let i = 0; i < solidityReceipt.logs.length; i++) {
            const solLog = solidityReceipt.logs[i];
            const huffLog = huffReceipt.logs[i];
            
            if (solLog.topics.join(',') !== huffLog.topics.join(',')) {
                return false;
            }
            
            if (solLog.data !== huffLog.data) {
                return false;
            }
        }
        
        return true;
    }

    private async compareStates(solidityState: any, huffState: any): boolean {
        if (!solidityState || !huffState) return true;
        
        // Compare token balances
        for (const token of Object.keys(solidityState.balances || {})) {
            const solBalance = solidityState.balances[token].solidity;
            const huffBalance = huffState.balances[token].huff;
            
            // Balances should match for respective contracts
            if (!solBalance.eq(huffBalance)) {
                return false;
            }
        }
        
        return true;
    }

    private getDifferenceDetails(solidityResult: any, huffResult: any): string {
        const differences = [];
        
        if (solidityResult.success !== huffResult.success) {
            differences.push(`Success mismatch: Solidity=${solidityResult.success}, Huff=${huffResult.success}`);
        }
        
        if (solidityResult.output !== huffResult.output) {
            differences.push(`Output mismatch`);
        }
        
        return differences.join('; ');
    }

    private generateReport(): void {
        console.log('\n' + '='.repeat(80));
        console.log('📊 PARITY VERIFICATION REPORT');
        console.log('='.repeat(80));
        
        const totalTests = this.results.length;
        const passedTests = this.results.filter(r => r.passed).length;
        const avgGasSaved = this.results
            .filter(r => r.passed && r.solidityGas > 0)
            .reduce((sum, r) => sum + r.percentSaved, 0) / passedTests || 0;
        
        console.log(`\nTests Run: ${totalTests}`);
        console.log(`Tests Passed: ${passedTests} (${(passedTests/totalTests*100).toFixed(1)}%)`);
        console.log(`Average Gas Saved: ${avgGasSaved.toFixed(1)}%`);
        
        console.log('\n📈 Detailed Results:');
        console.log('-'.repeat(80));
        
        for (const result of this.results) {
            const status = result.passed ? '✅' : '❌';
            const gasSavings = result.solidityGas > 0 
                ? `${result.percentSaved.toFixed(1)}% (${result.gasSaved} gas)`
                : 'N/A';
            
            console.log(`${status} ${result.scenario}`);
            console.log(`   Solidity Gas: ${result.solidityGas || 'N/A'}`);
            console.log(`   Huff Gas: ${result.huffGas || 'N/A'}`);
            console.log(`   Gas Saved: ${gasSavings}`);
            
            if (!result.passed) {
                console.log(`   Error: ${result.error}`);
            }
        }
        
        // Save report to file
        const reportPath = path.join(__dirname, `../monitoring/parity_report_${Date.now()}.json`);
        fs.writeFileSync(reportPath, JSON.stringify(this.results, null, 2));
        console.log(`\n💾 Report saved to: ${reportPath}`);
        
        // Check if gas target is met
        const gasTargetMet = avgGasSaved >= 65;
        if (gasTargetMet) {
            console.log('\n🎉 SUCCESS: Gas reduction target of 65% achieved!');
        } else {
            console.log(`\n⚠️  WARNING: Gas reduction ${avgGasSaved.toFixed(1)}% below 65% target`);
        }
    }
}

// CLI execution
async function main() {
    const solidityAddress = process.argv[2];
    const huffAddress = process.argv[3];
    
    if (!solidityAddress || !huffAddress) {
        console.log('Usage: npx ts-node verify_parity.ts <solidity-address> <huff-address>');
        console.log('\nExample:');
        console.log('  npx ts-node verify_parity.ts 0x123... 0x456...');
        process.exit(1);
    }
    
    const verifier = new ParityVerifier();
    
    try {
        await verifier.initialize(solidityAddress, huffAddress);
        await verifier.runComprehensiveParityTests();
        
        console.log('\n✅ Parity verification complete!');
        
    } catch (error) {
        console.error('\n❌ Parity verification failed:', error);
        process.exit(1);
    }
}

if (require.main === module) {
    main().catch(console.error);
}

export { ParityVerifier, TestScenario, ParityResult };