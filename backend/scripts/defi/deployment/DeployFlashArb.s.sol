
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Script.sol";
import "./FlashLoanArbitrage.sol";

contract DeployFlashArb is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);
        
        FlashLoanArbitrage arb = new FlashLoanArbitrage();
        console.log("Contract deployed at:", address(arb));
        
        // Check profitability
        (uint256 wpol, uint256 usdcNew, uint256 fee, uint256 profit) = 
            arb.checkProfitability(100 * 10**6);
        
        console.log("Expected WPOL:", wpol);
        console.log("Expected USDC.e:", usdcNew);
        console.log("Flash fee:", fee);
        console.log("Net profit:", profit);
        
        if (profit > 0) {
            console.log("Executing arbitrage...");
            arb.executeArbitrage(100 * 10**6);
        }
        
        vm.stopBroadcast();
    }
}
