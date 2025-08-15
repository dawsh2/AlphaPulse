const { ethers } = require("hardhat");
const fs = require("fs");
const path = require("path");

async function main() {
    console.log("🚀 Deploying Flash Arbitrage Contract...");
    
    // Polygon Aave V3 Pool Address Provider
    const AAVE_POOL_PROVIDER = "0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb";
    
    // Get deployer account
    const [deployer] = await ethers.getSigners();
    console.log("📋 Deploying from address:", deployer.address);
    
    // Check balance
    const balance = await deployer.getBalance();
    console.log("💰 Account balance:", ethers.utils.formatEther(balance), "MATIC");
    
    if (balance.lt(ethers.utils.parseEther("0.1"))) {
        throw new Error("Insufficient MATIC balance for deployment");
    }
    
    // Deploy contract
    const FlashArbitrage = await ethers.getContractFactory("FlashArbitrage");
    const flashArbitrage = await FlashArbitrage.deploy(AAVE_POOL_PROVIDER);
    
    await flashArbitrage.deployed();
    
    console.log("✅ Flash Arbitrage deployed to:", flashArbitrage.address);
    console.log("💾 Save this address for your bot config!");
    console.log("📋 Contract can now be called thousands of times for trading");
    
    // Save deployment info
    const deploymentInfo = {
        contractAddress: flashArbitrage.address,
        deploymentBlock: await ethers.provider.getBlockNumber(),
        deployer: deployer.address,
        network: network.name,
        aavePoolProvider: AAVE_POOL_PROVIDER,
        timestamp: new Date().toISOString()
    };
    
    // Save to file
    const deploymentsDir = path.join(__dirname, "../deployments");
    if (!fs.existsSync(deploymentsDir)) {
        fs.mkdirSync(deploymentsDir, { recursive: true });
    }
    
    const deploymentFile = path.join(deploymentsDir, `${network.name}-deployment.json`);
    fs.writeFileSync(deploymentFile, JSON.stringify(deploymentInfo, null, 2));
    
    console.log("📄 Deployment info saved to:", deploymentFile);
    console.log("\n📋 Deployment Summary:");
    console.log(JSON.stringify(deploymentInfo, null, 2));
    
    // Verify contract on Polygonscan (if API key is set)
    if (process.env.POLYGONSCAN_API_KEY) {
        console.log("\n🔍 Verifying contract on Polygonscan...");
        await new Promise(resolve => setTimeout(resolve, 30000)); // Wait for indexing
        
        try {
            await run("verify:verify", {
                address: flashArbitrage.address,
                constructorArguments: [AAVE_POOL_PROVIDER],
            });
            console.log("✅ Contract verified on Polygonscan");
        } catch (error) {
            console.error("❌ Verification failed:", error.message);
            console.log("You can verify manually with:");
            console.log(`npx hardhat verify --network ${network.name} ${flashArbitrage.address} "${AAVE_POOL_PROVIDER}"`);
        }
    }
    
    // Create bot config file
    const botConfig = {
        flashContractAddress: flashArbitrage.address,
        polygonRpcUrl: process.env.POLYGON_RPC_URL || "https://polygon-mainnet.public.blastapi.io",
        minProfitUSD: 15,
        maxGasPrice: 100,
        dexRouters: {
            quickswap: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff",
            sushiswap: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506",
            uniswap_v3: "0xE592427A0AEce92De3Edee1F18E0157C05861564"
        }
    };
    
    const configFile = path.join(__dirname, "../../services/arbitrage_bot/config.json");
    fs.writeFileSync(configFile, JSON.stringify(botConfig, null, 2));
    console.log("\n🤖 Bot config saved to:", configFile);
}

main()
    .then(() => process.exit(0))
    .catch((error) => {
        console.error(error);
        process.exit(1);
    });