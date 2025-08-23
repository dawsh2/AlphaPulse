// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";

contract DirectGasTest is Test {
    
    // Deployed Huff contract addresses
    address huffExtreme = 0x36E210D98064c3Cf764F7C6349E94bDc7D1b6b4D;
    address huffMEV = 0x10010Aa0548425E2Ffc86b57fDAba81Bceff9E27;
    address huffUltra = 0x08a853C53b6B1A8b12e904cc147e198dEba7E065;
    
    function test_DirectContractCalls() public {
        console.log("=== DIRECT CONTRACT GAS MEASUREMENT ===");
        
        // Test simple executeArbitrage call
        uint256 amount = 1000 * 1e6; // 1000 USDC
        bytes memory emptyData = "";
        
        // Test Extreme contract
        console.log("Testing Huff Extreme...");
        uint256 extremeGas = measureDirectCall(huffExtreme, amount, 1, emptyData);
        console.log("Extreme Gas:", extremeGas);
        
        // Test MEV contract
        console.log("Testing Huff MEV...");
        uint256 mevGas = measureDirectCall(huffMEV, amount, 1, emptyData);
        console.log("MEV Gas:", mevGas);
        
        // Test Ultra contract
        console.log("Testing Huff Ultra...");
        uint256 ultraGas = measureDirectCall(huffUltra, amount, 1, emptyData);
        console.log("Ultra Gas:", ultraGas);
        
        // Compare savings
        console.log("Gas savings vs Solidity (27,420):");
        console.log("Extreme:", 27420 - extremeGas);
        console.log("MEV:", 27420 - mevGas);
        console.log("Ultra:", 27420 - ultraGas);
    }
    
    function measureDirectCall(address contractAddr, uint256 amount, uint8 numSwaps, bytes memory data) internal returns (uint256) {
        uint256 gasBefore = gasleft();
        
        // Direct low-level call to measure gas
        contractAddr.call(
            abi.encodeWithSignature("executeArbitrage(uint256,uint8,bytes)", amount, numSwaps, data)
        );
        
        uint256 gasAfter = gasleft();
        return gasBefore - gasAfter;
    }
    
    function test_CompareAllContracts() public {
        console.log("=== COMPREHENSIVE GAS COMPARISON ===");
        
        uint256[] memory amounts = new uint256[](3);
        amounts[0] = 1000 * 1e6;
        amounts[1] = 10000 * 1e6;
        amounts[2] = 100000 * 1e6;
        
        for (uint i = 0; i < amounts.length; i++) {
            console.log("Amount:", amounts[i] / 1e6, "USDC");
            
            uint256 extremeGas = measureDirectCall(huffExtreme, amounts[i], 1, "");
            uint256 mevGas = measureDirectCall(huffMEV, amounts[i], 1, "");
            uint256 ultraGas = measureDirectCall(huffUltra, amounts[i], 1, "");
            
            console.log("  Extreme:", extremeGas, "gas");
            console.log("  MEV:", mevGas, "gas");
            console.log("  Ultra:", ultraGas, "gas");
            console.log("  Best:", ultraGas < mevGas && ultraGas < extremeGas ? "Ultra" : 
                       mevGas < extremeGas ? "MEV" : "Extreme");
            console.log("---");
        }
    }
}