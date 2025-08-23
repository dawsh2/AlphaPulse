// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface IUniswapV2Pair {
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    function swap(uint amount0Out, uint amount1Out, address to, bytes calldata data) external;
}

contract SimplestArbitrage {
    address private owner;
    
    address constant WPOL = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant USDC_OLD = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant USDC_NEW = 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359;
    
    address constant BUY_POOL = 0x380615F37993B5A96adF3D443b6E0Ac50a211998;
    address constant SELL_POOL = 0x6D9e8dbB2779853db00418D4DcF96F3987CFC9D2;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    // Simplest possible: You send USDC_OLD, get back USDC_NEW profit
    function executeArbitrage() external onlyOwner {
        // Get USDC_OLD balance sent to contract
        uint256 usdcOldBalance = IERC20(USDC_OLD).balanceOf(address(this));
        require(usdcOldBalance > 0, "No USDC_OLD");
        
        // Step 1: Send USDC_OLD to buy pool
        IERC20(USDC_OLD).transfer(BUY_POOL, usdcOldBalance);
        
        // Calculate WPOL output
        (uint112 r0, uint112 r1,) = IUniswapV2Pair(BUY_POOL).getReserves();
        uint256 wpolOut = (usdcOldBalance * 997 * r0) / (r1 * 1000 + usdcOldBalance * 997);
        
        // Execute swap for WPOL
        IUniswapV2Pair(BUY_POOL).swap(wpolOut, 0, address(this), "");
        
        // Step 2: Send WPOL to sell pool
        IERC20(WPOL).transfer(SELL_POOL, wpolOut);
        
        // Calculate USDC_NEW output
        (uint112 r0_2, uint112 r1_2,) = IUniswapV2Pair(SELL_POOL).getReserves();
        uint256 usdcNewOut = (wpolOut * 997 * r1_2) / (r0_2 * 1000 + wpolOut * 997);
        
        // Execute swap for USDC_NEW
        IUniswapV2Pair(SELL_POOL).swap(0, usdcNewOut, owner, "");  // Send directly to owner
    }
    
    // Withdraw any stuck tokens
    function withdraw(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
}