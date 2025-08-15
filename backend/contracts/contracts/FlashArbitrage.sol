// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {FlashLoanSimpleReceiverBase} from "@aave/core-v3/contracts/flashloan/base/FlashLoanSimpleReceiverBase.sol";
import {IPoolAddressesProvider} from "@aave/core-v3/contracts/interfaces/IPoolAddressesProvider.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

interface IUniswapV2Router02 {
    function swapExactTokensForTokens(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint256[] memory amounts);
    
    function getAmountsOut(
        uint256 amountIn,
        address[] calldata path
    ) external view returns (uint256[] memory amounts);
}

contract FlashArbitrage is FlashLoanSimpleReceiverBase {
    address private owner;
    uint256 private constant MAX_SLIPPAGE = 50; // 0.5% max slippage
    
    // Polygon DEX Routers
    address constant QUICKSWAP_ROUTER = 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff;
    address constant SUSHISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    address constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
    
    struct ArbitrageParams {
        address tokenIn;
        address tokenOut;
        address dexBuy;     // Router address for buying
        address dexSell;    // Router address for selling
        uint256 amountIn;
        uint256 minProfit;
    }
    
    event ArbitrageExecuted(
        address indexed tokenIn,
        address indexed tokenOut,
        uint256 amountIn,
        uint256 profit,
        address buyDex,
        address sellDex
    );
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner");
        _;
    }

    constructor(address _addressProvider) 
        FlashLoanSimpleReceiverBase(IPoolAddressesProvider(_addressProvider)) {
        owner = msg.sender;
    }

    function executeArbitrage(
        address tokenIn,
        address tokenOut,
        address dexBuy,
        address dexSell,
        uint256 amountIn,
        uint256 minProfit
    ) external onlyOwner {
        ArbitrageParams memory params = ArbitrageParams({
            tokenIn: tokenIn,
            tokenOut: tokenOut,
            dexBuy: dexBuy,
            dexSell: dexSell,
            amountIn: amountIn,
            minProfit: minProfit
        });
        
        bytes memory data = abi.encode(params);
        
        // Request flash loan from Aave
        POOL.flashLoanSimple(
            address(this),
            tokenIn,
            amountIn,
            data,
            0 // referral code
        );
    }

    function executeOperation(
        address asset,
        uint256 amount,
        uint256 premium,
        address initiator,
        bytes calldata params
    ) external override returns (bool) {
        // Ensure this was called by Aave pool
        require(msg.sender == address(POOL), "Invalid caller");
        require(initiator == address(this), "Invalid initiator");
        
        ArbitrageParams memory arbParams = abi.decode(params, (ArbitrageParams));
        
        // Step 1: Buy tokenOut on cheaper DEX
        IERC20(asset).approve(arbParams.dexBuy, amount);
        uint256 tokenOutReceived = _swapOnDEX(
            arbParams.dexBuy,
            asset,
            arbParams.tokenOut,
            amount
        );
        
        // Step 2: Sell tokenOut on expensive DEX
        IERC20(arbParams.tokenOut).approve(arbParams.dexSell, tokenOutReceived);
        uint256 tokenInReceived = _swapOnDEX(
            arbParams.dexSell,
            arbParams.tokenOut,
            asset,
            tokenOutReceived
        );
        
        // Step 3: Calculate profit and validate
        uint256 amountOwed = amount + premium;
        require(tokenInReceived > amountOwed, "Arbitrage not profitable");
        
        uint256 profit = tokenInReceived - amountOwed;
        require(profit >= arbParams.minProfit, "Profit below threshold");
        
        // Repay flash loan
        IERC20(asset).approve(address(POOL), amountOwed);
        
        // Send profit to owner
        if (profit > 0) {
            IERC20(asset).transfer(owner, profit);
        }
        
        emit ArbitrageExecuted(
            arbParams.tokenIn,
            arbParams.tokenOut,
            amount,
            profit,
            arbParams.dexBuy,
            arbParams.dexSell
        );
        
        return true;
    }
    
    function _swapOnDEX(
        address router,
        address tokenIn,
        address tokenOut,
        uint256 amountIn
    ) internal returns (uint256 amountOut) {
        address[] memory path = new address[](2);
        path[0] = tokenIn;
        path[1] = tokenOut;
        
        // Get expected output amount
        uint256[] memory expectedAmounts = IUniswapV2Router02(router).getAmountsOut(amountIn, path);
        uint256 expectedOut = expectedAmounts[1];
        
        // Calculate minimum acceptable amount (with slippage protection)
        uint256 minAmountOut = expectedOut * (10000 - MAX_SLIPPAGE) / 10000;
        
        // Execute swap
        uint256[] memory amounts = IUniswapV2Router02(router).swapExactTokensForTokens(
            amountIn,
            minAmountOut,
            path,
            address(this),
            block.timestamp + 300
        );
        
        return amounts[1];
    }
    
    // Emergency functions
    function emergencyWithdraw(address token) external onlyOwner {
        if (token == address(0)) {
            payable(owner).transfer(address(this).balance);
        } else {
            IERC20(token).transfer(owner, IERC20(token).balanceOf(address(this)));
        }
    }
    
    function updateOwner(address newOwner) external onlyOwner {
        require(newOwner != address(0), "Invalid owner");
        owner = newOwner;
    }
    
    receive() external payable {}
}