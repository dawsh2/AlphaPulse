
// Auto-generated arbitrage contract for:
// Pool1: 0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2 (QuickSwap)
// Pool2: 0x380615f37993b5a96adf3d443b6e0ac50a211998 (Dystopia-Stable)

pragma solidity ^0.8.0;

contract ArbitrageExecution {
    address constant POOL1 = 0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2;
    address constant POOL2 = 0x380615f37993b5a96adf3d443b6e0ac50a211998;
    address constant ROUTER1 = 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff;
    address constant ROUTER2 = 0xbE75Dd16D029c6B32B7aD57A0FD9C1c20Dd2862e;
    
    function execute(uint256 amount) external {
        // Route through appropriate routers
        
        // V2 swap via ROUTER1
        IUniswapV2Router(ROUTER1).swapExactTokensForTokens(
            amountIn, 0, path, address(this), block.timestamp
        );
        
        // Stable swap via Dystopia router
        IDystopiaRouter(ROUTER2).swapExactTokensForTokens(
            amountIn, 0, routes, address(this), block.timestamp
        );
    }
}
