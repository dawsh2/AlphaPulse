#!/usr/bin/env node

const { ethers } = require('ethers');
const fs = require('fs');
const path = require('path');
const readline = require('readline');

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

function question(query) {
    return new Promise(resolve => rl.question(query, resolve));
}

async function main() {
    console.log('🔐 Polygon Wallet Generator for AlphaPulse\n');
    console.log('This will generate a new Ethereum/Polygon compatible wallet.');
    console.log('⚠️  IMPORTANT: Keep your private key and mnemonic phrase SECURE!\n');

    // Generate new wallet
    const wallet = ethers.Wallet.createRandom();

    console.log('✨ NEW WALLET GENERATED\n');
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log('Address:', wallet.address);
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log('\n🔑 Private Key (KEEP SECRET!):\n', wallet.privateKey);
    console.log('\n📝 Mnemonic Phrase (BACKUP THIS!):\n', wallet.mnemonic.phrase);
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

    // Ask user what to do with the wallet
    const save = await question('Save to .env files? (y/n): ');
    
    if (save.toLowerCase() === 'y') {
        // Save to capital arbitrage bot .env
        const capitalArbEnvPath = path.join(__dirname, '../services/capital_arb_bot/.env');
        const capitalArbEnv = `# Generated Wallet for Capital Arbitrage Bot
# NEVER COMMIT THIS FILE TO GIT!
PRIVATE_KEY=${wallet.privateKey.substring(2)}  # Remove 0x prefix
WALLET_ADDRESS=${wallet.address}
POLYGON_RPC_URL=https://polygon-mainnet.public.blastapi.io
CHAIN_ID=137
MIN_PROFIT_USD=5.0
MAX_GAS_PRICE_GWEI=100.0
MAX_OPPORTUNITY_AGE_MS=5000
SIMULATION_MODE=true
MAX_TRADE_PERCENTAGE=0.5
SLIPPAGE_TOLERANCE=0.005
`;
        
        fs.writeFileSync(capitalArbEnvPath, capitalArbEnv);
        console.log('✅ Saved to:', capitalArbEnvPath);

        // Save to contracts .env
        const contractsEnvPath = path.join(__dirname, '../contracts/.env');
        const contractsEnv = `# Generated Wallet for Contract Deployment
# NEVER COMMIT THIS FILE TO GIT!
POLYGON_RPC_URL=https://polygon-mainnet.public.blastapi.io
MUMBAI_RPC_URL=https://rpc-mumbai.maticvigil.com
PRIVATE_KEY=${wallet.privateKey.substring(2)}  # Remove 0x prefix
POLYGONSCAN_API_KEY=your_polygonscan_api_key_here
FLASH_CONTRACT_ADDRESS=
MIN_PROFIT_USD=15
MAX_GAS_PRICE_GWEI=100
`;
        
        fs.writeFileSync(contractsEnvPath, contractsEnv);
        console.log('✅ Saved to:', contractsEnvPath);

        // Save backup to a secure location
        const backupPath = path.join(__dirname, `../wallet-backup-${Date.now()}.json`);
        const backupData = {
            address: wallet.address,
            privateKey: wallet.privateKey,
            mnemonic: wallet.mnemonic.phrase,
            createdAt: new Date().toISOString(),
            network: 'Polygon',
            chainId: 137
        };
        
        fs.writeFileSync(backupPath, JSON.stringify(backupData, null, 2));
        console.log('✅ Backup saved to:', backupPath);
        console.log('\n⚠️  IMPORTANT: Move the backup file to a secure location immediately!');
    }

    console.log('\n📋 Next Steps:');
    console.log('1. Save your mnemonic phrase in a secure location (password manager, etc.)');
    console.log('2. Fund your wallet:');
    console.log('   a. Send MATIC for gas fees (at least 5 MATIC recommended)');
    console.log('   b. Send USDC from Coinbase (use Polygon network!)');
    console.log('   c. Send other trading tokens as needed');
    console.log('\n3. To send from Coinbase:');
    console.log('   - Go to Coinbase → Send → Enter the address above');
    console.log('   - Select "Polygon" network (NOT Ethereum!)');
    console.log('   - Start with a small test amount first');
    console.log('\n4. Check your balance:');
    console.log('   node check-balance.js');
    console.log('\n5. Start trading (in simulation mode first):');
    console.log('   cd ../services/capital_arb_bot && cargo run');

    rl.close();
}

main().catch(console.error);