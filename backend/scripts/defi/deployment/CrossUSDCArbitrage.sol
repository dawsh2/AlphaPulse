// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface IUniswapV2Router {
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);
    
    function getAmountsOut(uint amountIn, address[] calldata path)
        external view returns (uint[] memory amounts);
}

contract CrossUSDCArbitrage {
    address private owner;
    
    // Polygon Mainnet addresses
    address constant WPOL = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant USDC_OLD = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant USDC_NEW = 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359;
    
    // Router addresses (you may need to identify which router each pool uses)
    address constant QUICKSWAP_ROUTER = 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff;
    address constant SUSHISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function executeArbitrage(
        address router1,
        address router2,
        uint256 amountIn,
        uint256 minProfit
    ) external onlyOwner {
        // Approve routers
        IERC20(USDC_OLD).approve(router1, amountIn);
        IERC20(WPOL).approve(router2, type(uint256).max);
        
        // Path 1: USDC_OLD -> WPOL
        address[] memory path1 = new address[](2);
        path1[0] = USDC_OLD;
        path1[1] = WPOL;
        
        // Execute first swap
        uint256 initialBalance = IERC20(USDC_OLD).balanceOf(address(this));
        uint256[] memory amounts1 = IUniswapV2Router(router1).swapExactTokensForTokens(
            amountIn,
            0, // Accept any amount of WPOL
            path1,
            address(this),
            block.timestamp + 300
        );
        
        uint256 wpolReceived = amounts1[1];
        
        // Path 2: WPOL -> USDC_NEW
        address[] memory path2 = new address[](2);
        path2[0] = WPOL;
        path2[1] = USDC_NEW;
        
        // Execute second swap
        uint256[] memory amounts2 = IUniswapV2Router(router2).swapExactTokensForTokens(
            wpolReceived,
            0, // Accept any amount of USDC_NEW
            path2,
            address(this),
            block.timestamp + 300
        );
        
        uint256 usdcNewReceived = amounts2[1];
        
        // Check profit (assuming USDC_OLD and USDC_NEW are 1:1)
        require(usdcNewReceived >= amountIn + minProfit, "Insufficient profit");
        
        // Transfer profits to owner
        IERC20(USDC_NEW).transfer(owner, usdcNewReceived);
        
        // If any USDC_OLD left, return it
        uint256 leftover = IERC20(USDC_OLD).balanceOf(address(this));
        if (leftover > 0) {
            IERC20(USDC_OLD).transfer(owner, leftover);
        }
    }
    
    function checkProfitability(
        address router1,
        address router2,
        uint256 amountIn
    ) external view returns (uint256 expectedOut, uint256 profit) {
        // Path 1: USDC_OLD -> WPOL
        address[] memory path1 = new address[](2);
        path1[0] = USDC_OLD;
        path1[1] = WPOL;
        
        uint256[] memory amounts1 = IUniswapV2Router(router1).getAmountsOut(amountIn, path1);
        uint256 wpolOut = amounts1[1];
        
        // Path 2: WPOL -> USDC_NEW
        address[] memory path2 = new address[](2);
        path2[0] = WPOL;
        path2[1] = USDC_NEW;
        
        uint256[] memory amounts2 = IUniswapV2Router(router2).getAmountsOut(wpolOut, path2);
        expectedOut = amounts2[1];
        
        if (expectedOut > amountIn) {
            profit = expectedOut - amountIn;
        } else {
            profit = 0;
        }
    }
    
    // Emergency withdrawal
    function withdraw(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
    
    receive() external payable {}
}