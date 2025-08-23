#!/usr/bin/env node

/**
 * Real Gas Testing Script
 * Tests actual gas usage by deploying to a mainnet fork
 * No fake numbers - only real blockchain measurements
 */

const { ethers } = require('ethers');
const { exec } = require('child_process');
const { promisify } = require('util');
const fs = require('fs').promises;
const path = require('path');

const execAsync = promisify(exec);

// Configuration for mainnet fork
const CONFIG = {
    // Fork URL - need to use a real Alchemy/Infura endpoint
    forkUrl: process.env.FORK_URL || 'https://polygon-mainnet.g.alchemy.com/v2/YOUR-KEY',
    
    // Test parameters
    testAmount: ethers.utils.parseUnits('1000', 6), // 1000 USDC
    
    // Known addresses on Polygon
    addresses: {
        USDC: '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174',
        WETH: '0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619',
        QUICKSWAP_ROUTER: '0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff',
        SUSHISWAP_ROUTER: '0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506',
        AAVE_POOL: '0x794a61358D6845594F94dc1DB02A252b5b4814aD',
    }
};

class RealGasTester {
    constructor() {
        this.provider = null;
        this.signer = null;
        this.solidityContract = null;
        this.huffContract = null;
        this.results = [];
    }

    async setup() {
        console.log('🔧 Setting up mainnet fork...');
        
        // Start local fork
        const forkProcess = exec(
            `npx hardhat node --fork ${CONFIG.forkUrl}`,
            { detached: true }
        );
        
        // Wait for fork to be ready
        await new Promise(resolve => setTimeout(resolve, 5000));
        
        // Connect to local fork
        this.provider = new ethers.providers.JsonRpcProvider('http://127.0.0.1:8545');
        
        // Get test signer (hardhat provides funded accounts)
        const accounts = await this.provider.listAccounts();
        this.signer = this.provider.getSigner(accounts[0]);
        
        console.log('  ✅ Fork ready at block', await this.provider.getBlockNumber());
        console.log('  ✅ Test account:', accounts[0]);
    }

    async deploySolidityContract() {
        console.log('\\n📝 Deploying Solidity contract...');
        
        // Read Solidity bytecode and ABI
        const solidityPath = path.join(__dirname, '../../contracts/FlashLoanArbitrage.sol');
        
        // Compile with solc
        const { stdout } = await execAsync(
            `solc --optimize --bin --abi ${solidityPath}`
        );
        
        // Parse compilation output
        const bytecodeMatch = stdout.match(/Binary:\\s*([0-9a-fA-F]+)/);
        const abiMatch = stdout.match(/Contract JSON ABI\\s*([\\s\\S]+?)(?=\\n\\n|$)/);
        
        if (!bytecodeMatch) {
            throw new Error('Failed to compile Solidity contract');
        }
        
        const bytecode = '0x' + bytecodeMatch[1];
        const abi = JSON.parse(abiMatch[1]);
        
        // Deploy contract
        const factory = new ethers.ContractFactory(abi, bytecode, this.signer);
        this.solidityContract = await factory.deploy();
        await this.solidityContract.deployed();
        
        console.log('  ✅ Solidity deployed at:', this.solidityContract.address);
        
        return this.solidityContract.address;
    }

    async deployHuffContract() {
        console.log('\\n🔨 Deploying Huff contract...');
        
        const huffPath = path.join(__dirname, '../contracts/huff/FlashLoanArbitrageReal.huff');
        const huffDir = path.dirname(huffPath);
        
        // Compile Huff contract
        const env = { ...process.env, PATH: `${process.env.HOME}/.huff/bin:${process.env.PATH}` };
        const { stdout: bytecode } = await execAsync(
            `cd ${huffDir} && huffc FlashLoanArbitrageReal.huff --bytecode`,
            { env }
        );
        
        // Deploy raw bytecode
        const tx = await this.signer.sendTransaction({
            data: '0x' + bytecode.trim()
        });
        
        const receipt = await tx.wait();
        this.huffContract = new ethers.Contract(
            receipt.contractAddress,
            this.solidityContract.interface, // Use same ABI
            this.signer
        );
        
        console.log('  ✅ Huff deployed at:', this.huffContract.address);
        console.log('  📊 Deployment gas used:', receipt.gasUsed.toString());
        
        return this.huffContract.address;
    }

    async setupTestEnvironment() {
        console.log('\\n💰 Setting up test environment...');
        
        // Impersonate a whale account that has USDC
        const whaleAddress = '0x075e72a5eDf65F0A5f44699c7654C1a76941Ddc8'; // Known USDC whale
        
        await this.provider.send('hardhat_impersonateAccount', [whaleAddress]);
        const whale = this.provider.getSigner(whaleAddress);
        
        // Transfer USDC to test contracts
        const usdc = new ethers.Contract(
            CONFIG.addresses.USDC,
            ['function transfer(address,uint256) returns (bool)'],
            whale
        );
        
        // Fund both contracts with some USDC for testing
        await usdc.transfer(this.solidityContract.address, CONFIG.testAmount);
        await usdc.transfer(this.huffContract.address, CONFIG.testAmount);
        
        console.log('  ✅ Test contracts funded with USDC');
    }

    async executeArbitrageTest(contract, name) {
        console.log(`\\n🚀 Testing ${name} arbitrage execution...`);
        
        try {
            // Execute arbitrage: USDC -> WETH -> USDC
            const tx = await contract.executeArbitrage(
                CONFIG.testAmount,
                CONFIG.addresses.QUICKSWAP_ROUTER,  // Buy on QuickSwap
                CONFIG.addresses.SUSHISWAP_ROUTER,  // Sell on SushiSwap
                CONFIG.addresses.WETH,              // Middle token
                0                                    // Min profit (for testing)
            );
            
            console.log(`  📝 Transaction hash: ${tx.hash}`);
            
            const receipt = await tx.wait();
            
            console.log(`  ⛽ Gas used: ${receipt.gasUsed.toString()}`);
            console.log(`  ✅ Status: ${receipt.status ? 'Success' : 'Failed'}`);
            
            return {
                name,
                gasUsed: receipt.gasUsed.toNumber(),
                success: receipt.status === 1,
                txHash: tx.hash
            };
            
        } catch (error) {
            console.error(`  ❌ Error: ${error.message}`);
            return {
                name,
                gasUsed: 0,
                success: false,
                error: error.message
            };
        }
    }

    async compareGasUsage() {
        console.log('\\n📊 Gas Usage Comparison:');
        console.log('=' .repeat(50));
        
        if (this.results.length !== 2) {
            console.log('❌ Need both test results to compare');
            return;
        }
        
        const [solidityResult, huffResult] = this.results;
        
        if (!solidityResult.success || !huffResult.success) {
            console.log('❌ Both tests must succeed for valid comparison');
            return;
        }
        
        const gasReduction = solidityResult.gasUsed - huffResult.gasUsed;
        const percentReduction = (gasReduction / solidityResult.gasUsed * 100).toFixed(2);
        
        console.log(`  Solidity gas: ${solidityResult.gasUsed.toLocaleString()}`);
        console.log(`  Huff gas: ${huffResult.gasUsed.toLocaleString()}`);
        console.log(`  Gas saved: ${gasReduction.toLocaleString()}`);
        console.log(`  Reduction: ${percentReduction}%`);
        
        console.log('\\n💡 Analysis:');
        
        if (percentReduction > 50) {
            console.log(`  🎉 Excellent! ${percentReduction}% gas reduction achieved`);
        } else if (percentReduction > 30) {
            console.log(`  ✅ Good! ${percentReduction}% gas reduction achieved`);
        } else if (percentReduction > 10) {
            console.log(`  📈 Moderate ${percentReduction}% gas reduction`);
        } else if (percentReduction > 0) {
            console.log(`  📉 Minor ${percentReduction}% gas reduction`);
        } else {
            console.log(`  ⚠️ No improvement or regression detected`);
        }
        
        // Calculate cost savings
        const gasPrice = 30; // gwei
        const maticPrice = 1.0; // USD
        const costSavings = (gasReduction * gasPrice * 1e-9 * maticPrice).toFixed(4);
        
        console.log(`\\n💰 Cost Analysis:`);
        console.log(`  Cost per tx saved: $${costSavings}`);
        console.log(`  Daily savings (100 tx): $${(costSavings * 100).toFixed(2)}`);
        console.log(`  Monthly savings (3000 tx): $${(costSavings * 3000).toFixed(2)}`);
        
        console.log('=' .repeat(50));
    }

    async runTests() {
        console.log('🧪 Real Gas Testing on Mainnet Fork');
        console.log('=' .repeat(50));
        
        try {
            // Setup
            await this.setup();
            
            // Deploy contracts
            await this.deploySolidityContract();
            await this.deployHuffContract();
            
            // Setup test environment
            await this.setupTestEnvironment();
            
            // Run tests
            const solidityResult = await this.executeArbitrageTest(
                this.solidityContract, 
                'Solidity'
            );
            this.results.push(solidityResult);
            
            const huffResult = await this.executeArbitrageTest(
                this.huffContract,
                'Huff'
            );
            this.results.push(huffResult);
            
            // Compare results
            await this.compareGasUsage();
            
            // Save results
            await this.saveResults();
            
        } catch (error) {
            console.error('\\n❌ Test failed:', error.message);
            throw error;
        }
    }

    async saveResults() {
        const resultsPath = path.join(__dirname, '../test_results/gas_comparison.json');
        
        const report = {
            timestamp: new Date().toISOString(),
            network: 'polygon-fork',
            blockNumber: await this.provider.getBlockNumber(),
            contracts: {
                solidity: this.solidityContract?.address,
                huff: this.huffContract?.address
            },
            results: this.results,
            comparison: this.results.length === 2 ? {
                gasReduction: this.results[0].gasUsed - this.results[1].gasUsed,
                percentReduction: ((this.results[0].gasUsed - this.results[1].gasUsed) / this.results[0].gasUsed * 100).toFixed(2)
            } : null
        };
        
        await fs.mkdir(path.dirname(resultsPath), { recursive: true });
        await fs.writeFile(resultsPath, JSON.stringify(report, null, 2));
        
        console.log(`\\n💾 Results saved to: ${resultsPath}`);
    }
}

// CLI execution
async function main() {
    const tester = new RealGasTester();
    
    try {
        await tester.runTests();
        process.exit(0);
    } catch (error) {
        console.error('Fatal error:', error);
        process.exit(1);
    }
}

if (require.main === module) {
    main().catch(console.error);
}

module.exports = { RealGasTester };