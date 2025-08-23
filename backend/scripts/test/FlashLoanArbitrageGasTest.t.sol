// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../src/FlashLoanArbitrage.sol";

contract FlashLoanArbitrageGasTest is Test {
    FlashLoanArbitrage public flashArb;
    
    function setUp() public {
        flashArb = new FlashLoanArbitrage();
    }
    
    /// @notice Test deployment gas cost
    function test_DeploymentGas() public {
        uint256 gasBefore = gasleft();
        FlashLoanArbitrage newContract = new FlashLoanArbitrage();
        uint256 gasAfter = gasleft();
        
        uint256 gasUsed = gasBefore - gasAfter;
        
        console.log("=== DEPLOYMENT GAS ANALYSIS ===");
        console.log("Deployment Gas Used:", gasUsed);
        console.log("Deployment Cost (30 gwei):", gasUsed * 30e9, "wei");
        console.log("Deployment Cost USD (MATIC $0.8): $", (gasUsed * 30e9 * 80) / 1e20);
    }
    
    /// @notice Test gas usage for different arbitrage amounts
    function test_ArbitrageGasUsage() public {
        console.log("=== ARBITRAGE GAS ANALYSIS ===");
        
        uint256[] memory amounts = new uint256[](4);
        amounts[0] = 100 * 1e6;   // 100 USDC
        amounts[1] = 500 * 1e6;   // 500 USDC  
        amounts[2] = 1000 * 1e6;  // 1000 USDC
        amounts[3] = 5000 * 1e6;  // 5000 USDC
        
        for (uint i = 0; i < amounts.length; i++) {
            vm.expectRevert();
            
            uint256 gasBefore = gasleft();
            flashArb.executeArbitrage(amounts[i]);
            uint256 gasAfter = gasleft();
            
            uint256 gasUsed = gasBefore - gasAfter;
            
            console.log("Flash Amount:", amounts[i] / 1e6, "USDC");
            console.log("Gas Used:", gasUsed);
            console.log("Cost (30 gwei):", gasUsed * 30e9, "wei");
            console.log("Cost USD:", (gasUsed * 30e9 * 80) / 1e20);
            console.log("---");
        }
    }
    
    /// @notice Test MEV competitiveness with different gas prices
    function test_MEVAnalysis() public {
        console.log("=== MEV COMPETITIVENESS ANALYSIS ===");
        
        uint256 estimatedGas = 250000; // Conservative estimate
        console.log("Estimated Gas Per Transaction:", estimatedGas);
        
        uint256[] memory gasPrices = new uint256[](5);
        gasPrices[0] = 20;   // Low congestion
        gasPrices[1] = 30;   // Normal
        gasPrices[2] = 50;   // High congestion  
        gasPrices[3] = 100;  // Extreme congestion
        gasPrices[4] = 200;  // MEV competition
        
        for (uint i = 0; i < gasPrices.length; i++) {
            uint256 costWei = estimatedGas * gasPrices[i] * 1e9;
            uint256 costUSD = (costWei * 80) / 1e20; // $0.8 MATIC
            uint256 minProfitBreakEven = costUSD + 1; // $1 minimum margin
            
            console.log("Gas Price:", gasPrices[i], "gwei");
            console.log("Cost USD: $", costUSD);
            console.log("Min Profitable Arb: $", minProfitBreakEven);
            console.log("---");
        }
    }
    
    /// @notice Test optimization scenarios
    function test_OptimizationScenarios() public {
        console.log("=== OPTIMIZATION SCENARIOS ===");
        
        uint256[] memory gasScenarios = new uint256[](4);
        gasScenarios[0] = 300000; // Current pessimistic
        gasScenarios[1] = 250000; // Current realistic
        gasScenarios[2] = 162000; // Huff 35% improvement
        gasScenarios[3] = 125000; // Huff 50% improvement
        
        uint256 gasPrice = 30; // 30 gwei
        uint256 dailyTxs = 100;
        
        console.log("Daily Transactions:", dailyTxs);
        console.log("Gas Price:", gasPrice, "gwei");
        
        for (uint i = 0; i < gasScenarios.length; i++) {
            uint256 costPerTx = (gasScenarios[i] * gasPrice * 1e9 * 80) / 1e20; // USD
            uint256 dailyCost = costPerTx * dailyTxs;
            uint256 annualCost = dailyCost * 365;
            
            console.log("Scenario", i + 1, "- Gas:", gasScenarios[i]);
            console.log("Cost per TX: $", costPerTx);
            console.log("Daily Cost: $", dailyCost);
            console.log("Annual Cost: $", annualCost);
            console.log("---");
        }
        
        // Calculate annual savings with optimization
        uint256 currentAnnualCost = (300000 * gasPrice * 1e9 * 80 * dailyTxs * 365) / 1e20;
        uint256 optimizedAnnualCost = (125000 * gasPrice * 1e9 * 80 * dailyTxs * 365) / 1e20;
        uint256 annualSavings = currentAnnualCost - optimizedAnnualCost;
        
        console.log("Annual Savings with 50% Huff Optimization: $", annualSavings);
    }
    
    /// @notice Test profitability thresholds
    function test_ProfitabilityThresholds() public {
        console.log("=== PROFITABILITY THRESHOLDS ===");
        
        uint256[] memory scenarios = new uint256[](4);
        scenarios[0] = 300000; // Current
        scenarios[1] = 200000; // 33% improvement
        scenarios[2] = 162000; // 46% improvement  
        scenarios[3] = 125000; // 58% improvement
        
        uint256 gasPrice = 30; // 30 gwei
        
        for (uint i = 0; i < scenarios.length; i++) {
            uint256 gasCostUSD = (scenarios[i] * gasPrice * 1e9 * 80) / 1e20;
            uint256 breakEven = gasCostUSD + 1; // $1 margin
            uint256 profitable10Percent = (gasCostUSD * 110) / 100; // 10% margin
            
            console.log("Gas Usage:", scenarios[i]);
            console.log("Gas Cost: $", gasCostUSD);
            console.log("Break-even + $1: $", breakEven);
            console.log("10% Margin: $", profitable10Percent);
            console.log("---");
        }
    }
}