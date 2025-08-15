#!/usr/bin/env node

const { ethers } = require('ethers');
const fs = require('fs');
const path = require('path');

// Token addresses on Polygon
const TOKENS = {
    WMATIC: { address: '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270', decimals: 18, symbol: 'WMATIC' },
    USDC: { address: '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174', decimals: 6, symbol: 'USDC' },
    USDT: { address: '0xc2132D05D31c914a87C6611C10748AEb04B58e8F', decimals: 6, symbol: 'USDT' },
    WETH: { address: '0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619', decimals: 18, symbol: 'WETH' },
    DAI: { address: '0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063', decimals: 18, symbol: 'DAI' }
};

const ERC20_ABI = [
    'function balanceOf(address owner) view returns (uint256)',
    'function symbol() view returns (string)',
    'function decimals() view returns (uint8)'
];

async function main() {
    console.log('💰 Polygon Wallet Balance Checker\n');

    // Try to load from multiple possible .env locations
    const envPaths = [
        path.join(__dirname, '../services/capital_arb_bot/.env'),
        path.join(__dirname, '../contracts/.env'),
        path.join(__dirname, '../../.env')
    ];

    let privateKey, walletAddress;
    
    for (const envPath of envPaths) {
        if (fs.existsSync(envPath)) {
            const envContent = fs.readFileSync(envPath, 'utf8');
            const privateKeyMatch = envContent.match(/PRIVATE_KEY=([a-fA-F0-9]{64})/);
            const addressMatch = envContent.match(/WALLET_ADDRESS=(0x[a-fA-F0-9]{40})/);
            
            if (privateKeyMatch) {
                privateKey = '0x' + privateKeyMatch[1];
            }
            if (addressMatch) {
                walletAddress = addressMatch[1];
            }
            
            if (privateKey && walletAddress) {
                console.log(`📁 Loaded wallet from: ${envPath}\n`);
                break;
            }
        }
    }

    if (!privateKey || !walletAddress) {
        console.error('❌ Could not find wallet configuration.');
        console.error('   Run generate-wallet.js first or check your .env files');
        process.exit(1);
    }

    // Connect to Polygon
    const provider = new ethers.providers.JsonRpcProvider('https://polygon-mainnet.public.blastapi.io');
    
    console.log('🔗 Connected to Polygon Mainnet');
    console.log('📍 Wallet Address:', walletAddress);
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

    try {
        // Check native MATIC balance
        const maticBalance = await provider.getBalance(walletAddress);
        const maticFormatted = ethers.utils.formatEther(maticBalance);
        console.log(`MATIC: ${maticFormatted} MATIC ($${(parseFloat(maticFormatted) * 0.8).toFixed(2)} @ $0.80)`);

        // Check token balances
        for (const [name, token] of Object.entries(TOKENS)) {
            const contract = new ethers.Contract(token.address, ERC20_ABI, provider);
            const balance = await contract.balanceOf(walletAddress);
            const formatted = ethers.utils.formatUnits(balance, token.decimals);
            
            if (parseFloat(formatted) > 0) {
                console.log(`${token.symbol}: ${formatted}`);
            }
        }

        console.log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
        
        // Check if wallet is ready for trading
        const maticValue = parseFloat(maticFormatted);
        if (maticValue < 1) {
            console.log('\n⚠️  Low MATIC balance! You need MATIC for gas fees.');
            console.log('   Recommended: Send at least 5 MATIC for trading');
        } else if (maticValue < 5) {
            console.log('\n⚠️  MATIC balance is low. Consider adding more for active trading.');
        } else {
            console.log('\n✅ MATIC balance sufficient for trading');
        }

        // Check for trading tokens
        const usdcContract = new ethers.Contract(TOKENS.USDC.address, ERC20_ABI, provider);
        const usdcBalance = await usdcContract.balanceOf(walletAddress);
        const usdcFormatted = parseFloat(ethers.utils.formatUnits(usdcBalance, 6));
        
        if (usdcFormatted === 0) {
            console.log('\n📌 To start trading:');
            console.log('   1. Send USDC from Coinbase to:', walletAddress);
            console.log('   2. Make sure to select "Polygon" network (not Ethereum!)');
            console.log('   3. Start with a small test amount (e.g., $10)');
        } else {
            console.log(`\n✅ Ready to trade with $${usdcFormatted.toFixed(2)} USDC`);
        }

        // Show Polygonscan link
        console.log('\n🔍 View on Polygonscan:');
        console.log(`   https://polygonscan.com/address/${walletAddress}`);

    } catch (error) {
        console.error('❌ Error checking balances:', error.message);
    }
}

main().catch(console.error);