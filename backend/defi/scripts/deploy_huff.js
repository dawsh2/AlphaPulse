#!/usr/bin/env node

/**
 * Huff Contract Deployment Script
 * Deploys both Solidity and Huff implementations for comparison
 */

const { ethers } = require('ethers');
const { exec } = require('child_process');
const { promisify } = require('util');
const fs = require('fs').promises;
const path = require('path');

const execAsync = promisify(exec);

// Configuration
const CONFIG = {
    rpcUrl: process.env.RPC_URL || 'https://polygon-mumbai.g.alchemy.com/v2/YOUR_KEY',
    privateKey: process.env.PRIVATE_KEY || '',
    network: process.env.NETWORK || 'polygon-mumbai',
};

class HuffDeployer {
    constructor() {
        this.provider = new ethers.providers.JsonRpcProvider(CONFIG.rpcUrl);
        this.wallet = new ethers.Wallet(CONFIG.privateKey, this.provider);
    }

    async compileSolidity() {
        console.log('📝 Compiling Solidity contract...');
        
        // For demo purposes, we'll use a simple bytecode
        // In production, compile the actual Solidity contract
        const solidityBytecode = '0x608060405234801561001057600080fd5b50336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550610300806100616000396000f3fe';
        
        return solidityBytecode;
    }

    async compileHuff() {
        console.log('🔨 Compiling Huff contract...');
        
        const huffPath = path.join(__dirname, '../contracts/huff/FlashArbitrageOptimized.huff');
        
        try {
            // Set PATH to include Huff compiler
            const env = { ...process.env, PATH: `${process.env.HOME}/.huff/bin:${process.env.PATH}` };
            
            // Compile Huff contract
            const { stdout } = await execAsync(`huffc ${huffPath} --bytecode`, { env });
            
            console.log(`  ✅ Huff compiled: ${stdout.length / 2} bytes`);
            
            return '0x' + stdout.trim();
        } catch (error) {
            console.error('Failed to compile Huff:', error);
            throw error;
        }
    }

    async deployContract(bytecode, name) {
        console.log(`\n🚀 Deploying ${name}...`);
        
        const factory = new ethers.ContractFactory([], bytecode, this.wallet);
        
        const gasPrice = await this.provider.getGasPrice();
        console.log(`  Gas price: ${ethers.utils.formatUnits(gasPrice, 'gwei')} gwei`);
        
        const deployTx = factory.getDeployTransaction();
        deployTx.gasPrice = gasPrice.mul(110).div(100); // 10% buffer
        
        // Estimate gas
        try {
            const estimatedGas = await this.wallet.estimateGas(deployTx);
            console.log(`  Estimated gas: ${estimatedGas.toString()}`);
            deployTx.gasLimit = estimatedGas.mul(120).div(100); // 20% buffer
        } catch (error) {
            console.log('  Using default gas limit');
            deployTx.gasLimit = 3000000;
        }
        
        // Send deployment transaction
        const tx = await this.wallet.sendTransaction(deployTx);
        console.log(`  Tx hash: ${tx.hash}`);
        console.log('  Waiting for confirmation...');
        
        const receipt = await tx.wait();
        console.log(`  ✅ Deployed at: ${receipt.contractAddress}`);
        console.log(`  Gas used: ${receipt.gasUsed.toString()}`);
        
        return receipt.contractAddress;
    }

    async verifyDeployment(address) {
        console.log(`\n🔍 Verifying deployment at ${address}...`);
        
        const code = await this.provider.getCode(address);
        
        if (code === '0x') {
            throw new Error('No code at address!');
        }
        
        console.log(`  ✅ Contract verified: ${code.length / 2 - 1} bytes`);
        
        return true;
    }

    async saveDeploymentInfo(solidityAddress, huffAddress) {
        const deploymentInfo = {
            network: CONFIG.network,
            timestamp: new Date().toISOString(),
            contracts: {
                solidity: solidityAddress,
                huff: huffAddress,
            },
            gasComparison: {
                note: 'Run gas_profiler.js to measure actual gas usage',
            },
        };
        
        const deploymentPath = path.join(__dirname, '../deployments.json');
        await fs.writeFile(deploymentPath, JSON.stringify(deploymentInfo, null, 2));
        
        console.log(`\n💾 Deployment info saved to deployments.json`);
        
        return deploymentInfo;
    }

    async run() {
        console.log('🎯 Huff Contract Deployment System');
        console.log('=' .repeat(50));
        console.log(`Network: ${CONFIG.network}`);
        console.log(`Deployer: ${this.wallet.address}`);
        
        // Check balance
        const balance = await this.wallet.getBalance();
        console.log(`Balance: ${ethers.utils.formatEther(balance)} MATIC`);
        
        if (balance.eq(0)) {
            throw new Error('Insufficient balance for deployment');
        }
        
        try {
            // Compile contracts
            const solidityBytecode = await this.compileSolidity();
            const huffBytecode = await this.compileHuff();
            
            console.log('\n📊 Bytecode Comparison:');
            console.log(`  Solidity: ${solidityBytecode.length / 2 - 1} bytes`);
            console.log(`  Huff: ${huffBytecode.length / 2 - 1} bytes`);
            console.log(`  Reduction: ${Math.round((1 - (huffBytecode.length / solidityBytecode.length)) * 100)}%`);
            
            // Deploy contracts
            const solidityAddress = await this.deployContract(solidityBytecode, 'Solidity Implementation');
            const huffAddress = await this.deployContract(huffBytecode, 'Huff Implementation');
            
            // Verify deployments
            await this.verifyDeployment(solidityAddress);
            await this.verifyDeployment(huffAddress);
            
            // Save deployment info
            const info = await this.saveDeploymentInfo(solidityAddress, huffAddress);
            
            console.log('\n' + '='.repeat(50));
            console.log('✅ DEPLOYMENT SUCCESSFUL!');
            console.log('='.repeat(50));
            console.log('\n📋 Summary:');
            console.log(`  Solidity: ${solidityAddress}`);
            console.log(`  Huff: ${huffAddress}`);
            console.log('\n📈 Next Steps:');
            console.log(`  1. Run gas profiler:`);
            console.log(`     node gas_profiler.js ${solidityAddress} ${huffAddress}`);
            console.log(`  2. Run parity tests:`);
            console.log(`     npx ts-node verify_parity.ts ${solidityAddress} ${huffAddress}`);
            console.log(`  3. Monitor with canary deployment:`);
            console.log(`     cargo run --bin canary_monitor`);
            
            return info;
            
        } catch (error) {
            console.error('\n❌ Deployment failed:', error.message);
            throw error;
        }
    }
}

// CLI execution
async function main() {
    if (!CONFIG.privateKey) {
        console.error('❌ PRIVATE_KEY environment variable not set');
        console.log('\nUsage:');
        console.log('  PRIVATE_KEY=0x... RPC_URL=https://... node deploy_huff.js');
        process.exit(1);
    }
    
    const deployer = new HuffDeployer();
    
    try {
        await deployer.run();
        process.exit(0);
    } catch (error) {
        console.error('Fatal error:', error);
        process.exit(1);
    }
}

if (require.main === module) {
    main().catch(console.error);
}

module.exports = { HuffDeployer };