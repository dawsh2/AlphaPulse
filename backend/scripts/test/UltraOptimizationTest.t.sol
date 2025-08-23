// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";

contract UltraOptimizationTest is Test {
    
    // Deployed Huff contract addresses
    address huffMEV = 0x10010Aa0548425E2Ffc86b57fDAba81Bceff9E27;
    address huffUltra = 0x08a853C53b6B1A8b12e904cc147e198dEba7E065;
    
    // Aave Pool address (for executeOperation callback)
    address constant AAVE_POOL = 0x794a61358D6845594F94dc1DB02A252b5b4814aD;
    
    function test_ExecuteOperationComparison() public {
        console.log("=== EXECUTE OPERATION CALLBACK COMPARISON ===");
        
        // Test the executeOperation callback where real optimizations happen
        uint256 amount = 10000 * 1e6; // 10,000 USDC
        uint256 premium = 5 * 1e6;    // 5 USDC premium
        
        // Build complex swap data for 1, 2, and 3 swaps to test unrolled loops
        
        // Single swap test
        console.log("Testing single swap:");
        bytes memory singleSwapData = buildSwapData(1);
        uint256 mevGas1 = measureExecuteOperation(huffMEV, amount, premium, singleSwapData);
        uint256 ultraGas1 = measureExecuteOperation(huffUltra, amount, premium, singleSwapData);
        console.log("MEV single swap:", mevGas1, "gas");
        console.log("Ultra single swap:", ultraGas1, "gas");
        console.log("Ultra savings:", mevGas1 - ultraGas1, "gas");
        console.log("");
        
        // Double swap test
        console.log("Testing double swap:");
        bytes memory doubleSwapData = buildSwapData(2);
        uint256 mevGas2 = measureExecuteOperation(huffMEV, amount, premium, doubleSwapData);
        uint256 ultraGas2 = measureExecuteOperation(huffUltra, amount, premium, doubleSwapData);
        console.log("MEV double swap:", mevGas2, "gas");
        console.log("Ultra double swap:", ultraGas2, "gas");
        console.log("Ultra savings:", mevGas2 - ultraGas2, "gas");
        console.log("");
        
        // Triple swap test (should show maximum Ultra optimization benefit)
        console.log("Testing triple swap:");
        bytes memory tripleSwapData = buildSwapData(3);
        uint256 mevGas3 = measureExecuteOperation(huffMEV, amount, premium, tripleSwapData);
        uint256 ultraGas3 = measureExecuteOperation(huffUltra, amount, premium, tripleSwapData);
        console.log("MEV triple swap:", mevGas3, "gas");
        console.log("Ultra triple swap:", ultraGas3, "gas");
        console.log("Ultra savings:", mevGas3 - ultraGas3, "gas");
        console.log("");
        
        console.log("TOTAL ULTRA OPTIMIZATIONS:");
        uint256 totalSavings = (mevGas1 - ultraGas1) + (mevGas2 - ultraGas2) + (mevGas3 - ultraGas3);
        console.log("Total gas saved:", totalSavings);
        console.log("Average savings per operation:", totalSavings / 3);
    }
    
    function measureExecuteOperation(address contractAddr, uint256 amount, uint256 premium, bytes memory params) internal returns (uint256) {
        // Prank as Aave pool to call executeOperation
        vm.prank(AAVE_POOL);
        
        uint256 gasBefore = gasleft();
        
        // Call executeOperation which contains the real arbitrage logic
        contractAddr.call(
            abi.encodeWithSignature(
                "executeOperation(address,uint256,uint256,address,bytes)",
                0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174, // USDC
                amount,
                premium,
                AAVE_POOL,
                params
            )
        );
        
        uint256 gasAfter = gasleft();
        return gasBefore - gasAfter;
    }
    
    function buildSwapData(uint8 numSwaps) internal pure returns (bytes memory) {
        // Simple swap data to trigger unrolled loop optimizations
        if (numSwaps == 1) {
            return abi.encode(uint8(1), uint256(100));
        } else if (numSwaps == 2) {
            return abi.encode(uint8(2), uint256(200));
        } else {
            return abi.encode(uint8(3), uint256(300));
        }
    }
    
    function test_MemoryOptimizationBenefit() public {
        console.log("=== MEMORY OPTIMIZATION TEST ===");
        
        // Test memory layout optimization with larger swap data
        uint256 amount = 50000 * 1e6; // 50,000 USDC
        uint256 premium = 25 * 1e6;   // 25 USDC premium
        
        // Build data that will stress test memory optimizations
        bytes memory complexData = buildComplexSwapData();
        
        uint256 mevGas = measureExecuteOperation(huffMEV, amount, premium, complexData);
        uint256 ultraGas = measureExecuteOperation(huffUltra, amount, premium, complexData);
        
        console.log("MEV complex operation:", mevGas, "gas");
        console.log("Ultra complex operation:", ultraGas, "gas");
        console.log("Memory optimization savings:", mevGas - ultraGas, "gas");
        
        // Calculate percentage improvement
        if (mevGas > ultraGas) {
            uint256 improvement = ((mevGas - ultraGas) * 100) / mevGas;
            console.log("Ultra improvement:", improvement, "%");
        }
    }
    
    function buildComplexSwapData() internal pure returns (bytes memory) {
        // Build data with multiple V2 and V3 swaps to trigger all optimizations
        return abi.encode(
            uint8(3), // numSwaps
            0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174, // flashToken (USDC)
            uint256(0), // swapData offset
            uint256(0), // minProfit
            
            // Swap 1: USDC -> WMATIC (V2)
            0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff, // router
            uint8(2),   // poolType (V2)
            0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174, // tokenIn (USDC)
            0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270, // tokenOut (WMATIC)
            uint24(0),  // fee
            uint256(0), // minAmountOut
            
            // Swap 2: WMATIC -> WETH (V3)
            0xE592427A0AEce92De3Edee1F18E0157C05861564, // Uniswap V3 router
            uint8(3),   // poolType (V3)
            0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270, // tokenIn (WMATIC)
            0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619, // tokenOut (WETH)
            uint24(3000), // fee (0.3%)
            uint256(0), // minAmountOut
            
            // Swap 3: WETH -> USDC (V2)
            0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff, // router
            uint8(2),   // poolType (V2)
            0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619, // tokenIn (WETH)
            0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174, // tokenOut (USDC)
            uint24(0),  // fee
            uint256(0)  // minAmountOut
        );
    }
}