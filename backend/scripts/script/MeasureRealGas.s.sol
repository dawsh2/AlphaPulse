// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";

contract MeasureRealGas is Script {
    
    // Mumbai testnet addresses (will be updated after deployment)
    address huffExtreme = 0x1234567890123456789012345678901234567890; // PLACEHOLDER
    address huffMEV = 0x1234567890123456789012345678901234567891;     // PLACEHOLDER  
    address huffUltra = 0x1234567890123456789012345678901234567892;   // PLACEHOLDER
    
    // Mumbai testnet token addresses
    address constant USDC = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant WMATIC = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant WETH = 0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619;
    
    // QuickSwap router on Mumbai
    address constant QUICKSWAP_ROUTER = 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff;
    
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);
        
        console.log("=== MEASURING REAL EXECUTION GAS ON MUMBAI ===");
        
        // Test different arbitrage amounts
        uint256[] memory amounts = new uint256[](4);
        amounts[0] = 100 * 1e6;    // 100 USDC
        amounts[1] = 1000 * 1e6;   // 1,000 USDC  
        amounts[2] = 5000 * 1e6;   // 5,000 USDC
        amounts[3] = 10000 * 1e6;  // 10,000 USDC
        
        for (uint i = 0; i < amounts.length; i++) {
            uint256 amount = amounts[i];
            console.log("Testing amount:", amount / 1e6, "USDC");
            
            // Test Extreme contract
            if (huffExtreme != address(0x1234567890123456789012345678901234567890)) {
                console.log("Testing Huff Extreme...");
                uint256 gasUsed = measureExecutionGas(huffExtreme, amount, 1);
                console.log("Extreme Gas Used:", gasUsed);
            }
            
            // Test MEV contract
            if (huffMEV != address(0x1234567890123456789012345678901234567891)) {
                console.log("Testing Huff MEV...");
                uint256 gasUsed = measureExecutionGas(huffMEV, amount, 2);
                console.log("MEV Gas Used:", gasUsed);
            }
            
            // Test Ultra contract
            if (huffUltra != address(0x1234567890123456789012345678901234567892)) {
                console.log("Testing Huff Ultra...");
                uint256 gasUsed = measureExecutionGas(huffUltra, amount, 3);
                console.log("Ultra Gas Used:", gasUsed);
            }
            
            console.log("---");
        }
        
        vm.stopBroadcast();
    }
    
    function measureExecutionGas(address contractAddr, uint256 amount, uint8 numSwaps) internal returns (uint256) {
        // Build swap data for simple USDC -> WMATIC -> USDC arbitrage
        bytes memory swapData = buildSimpleSwapData(amount);
        
        uint256 gasBefore = gasleft();
        
        try this.executeArbitrage(contractAddr, amount, numSwaps, swapData) {
            // Successful execution
        } catch {
            // Expected to fail due to liquidity/slippage, but we got gas measurement
        }
        
        uint256 gasAfter = gasleft();
        return gasBefore - gasAfter;
    }
    
    function executeArbitrage(address contractAddr, uint256 amount, uint8 numSwaps, bytes memory swapData) external {
        (bool success, ) = contractAddr.call(
            abi.encodeWithSignature("executeArbitrage(uint256,uint8,bytes)", amount, numSwaps, swapData)
        );
        require(success, "Arbitrage execution failed");
    }
    
    function buildSimpleSwapData(uint256 amount) internal pure returns (bytes memory) {
        // Simple 2-hop arbitrage: USDC -> WMATIC -> USDC
        // Swap 1: USDC -> WMATIC on QuickSwap V2
        // Swap 2: WMATIC -> USDC on QuickSwap V2
        
        return abi.encode(
            // Swap 1 data
            QUICKSWAP_ROUTER,     // router
            uint8(2),             // poolType (V2)
            USDC,                 // tokenIn
            WMATIC,               // tokenOut
            uint24(3000),         // fee (not used for V2)
            amount * 99 / 100,    // minAmountOut (1% slippage)
            
            // Swap 2 data  
            QUICKSWAP_ROUTER,     // router
            uint8(2),             // poolType (V2)
            WMATIC,               // tokenIn
            USDC,                 // tokenOut
            uint24(3000),         // fee (not used for V2)
            amount * 101 / 100    // minAmountOut (1% profit target)
        );
    }
    
    // Helper function to update contract addresses after deployment
    function setContractAddresses(address _extreme, address _mev, address _ultra) external {
        huffExtreme = _extreme;
        huffMEV = _mev;
        huffUltra = _ultra;
    }
}