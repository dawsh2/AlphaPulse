// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";

contract SimpleUltraTest is Test {
    
    address huffMEV = 0x10010Aa0548425E2Ffc86b57fDAba81Bceff9E27;
    address huffUltra = 0x08a853C53b6B1A8b12e904cc147e198dEba7E065;
    address aavePool = 0x794a61358D6845594F94dc1DB02A252b5b4814aD;
    
    function test_UltraVsMEV() public {
        console.log("=== ULTRA VS MEV OPTIMIZATION TEST ===");
        
        // Test executeOperation callback where real optimizations happen
        vm.prank(aavePool);
        
        uint256 mevGas = measureGas(huffMEV);
        uint256 ultraGas = measureGas(huffUltra);
        
        console.log("MEV Gas:", mevGas);
        console.log("Ultra Gas:", ultraGas);
        
        if (ultraGas < mevGas) {
            console.log("Ultra saves:", mevGas - ultraGas, "gas");
            console.log("Ultra is", ((mevGas - ultraGas) * 100) / mevGas, "% better");
        } else if (mevGas < ultraGas) {
            console.log("MEV saves:", ultraGas - mevGas, "gas");  
            console.log("MEV is", ((ultraGas - mevGas) * 100) / ultraGas, "% better");
        } else {
            console.log("Same gas usage - optimizations not triggered");
        }
    }
    
    function measureGas(address contractAddr) internal returns (uint256) {
        uint256 gasBefore = gasleft();
        
        contractAddr.call(
            abi.encodeWithSignature(
                "executeOperation(address,uint256,uint256,address,bytes)",
                0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174,
                1000000000,
                5000000,
                aavePool,
                abi.encode(uint8(3), uint256(0))
            )
        );
        
        uint256 gasAfter = gasleft();
        return gasBefore - gasAfter;
    }
}