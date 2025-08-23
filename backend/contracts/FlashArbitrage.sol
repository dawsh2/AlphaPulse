// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@aave/core-v3/contracts/flashloan/base/FlashLoanSimpleReceiverBase.sol";
import "@aave/core-v3/contracts/interfaces/IPoolAddressesProvider.sol";
import "@aave/core-v3/contracts/interfaces/IPool.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

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

interface IUniswapV3Router {
    struct ExactInputSingleParams {
        address tokenIn;
        address tokenOut;
        uint24 fee;
        address recipient;
        uint256 deadline;
        uint256 amountIn;
        uint256 amountOutMinimum;
        uint160 sqrtPriceLimitX96;
    }
    
    function exactInputSingle(ExactInputSingleParams calldata params)
        external payable returns (uint256 amountOut);
}

/**
 * @title FlashArbitrage
 * @dev Executes arbitrage between DEXs using Aave V3 flash loans
 */
contract FlashArbitrage is FlashLoanSimpleReceiverBase {
    address private owner;
    
    // Polygon Mainnet Addresses
    address constant AAVE_POOL_PROVIDER = 0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb;
    
    // DEX Routers
    address constant QUICKSWAP_ROUTER = 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff;
    address constant SUSHISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    address constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
    
    // Common tokens
    address constant WMATIC = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant USDC = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant USDT = 0xc2132D05D31c914a87C6611C10748AEb04B58e8F;
    address constant WETH = 0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619;
    
    struct ArbitrageParams {
        address tokenIn;
        address tokenOut;
        address buyRouter;    // Router to buy tokenOut with tokenIn
        address sellRouter;   // Router to sell tokenOut for tokenIn
        uint24 buyFee;        // V3 fee tier (0 for V2)
        uint24 sellFee;       // V3 fee tier (0 for V2)
        uint256 amountIn;     // Flash loan amount
        uint256 minProfit;    // Minimum profit required
    }
    
    event ArbitrageExecuted(
        address indexed tokenIn,
        address indexed tokenOut,
        uint256 amountIn,
        uint256 profit
    );
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() FlashLoanSimpleReceiverBase(IPoolAddressesProvider(AAVE_POOL_PROVIDER)) {
        owner = msg.sender;
    }
    
    /**
     * @dev Execute arbitrage using flash loan
     * @param params Arbitrage parameters
     */
    function executeArbitrage(ArbitrageParams calldata params) external onlyOwner {
        // Validate parameters
        require(params.amountIn > 0, "Invalid amount");
        require(params.tokenIn != params.tokenOut, "Same token");
        
        // Encode parameters for flash loan callback
        bytes memory data = abi.encode(params);
        
        // Request flash loan
        POOL.flashLoanSimple(
            address(this),
            params.tokenIn,
            params.amountIn,
            data,
            0 // referralCode
        );
    }
    
    /**
     * @dev Flash loan callback - executes arbitrage logic
     */
    function executeOperation(
        address asset,
        uint256 amount,
        uint256 premium,
        address initiator,
        bytes calldata params
    ) external override returns (bool) {
        require(msg.sender == address(POOL), "Only Aave pool");
        require(initiator == address(this), "Invalid initiator");
        
        // Decode parameters
        ArbitrageParams memory arbParams = abi.decode(params, (ArbitrageParams));
        
        // Execute arbitrage
        uint256 profit = _executeArbitrage(arbParams);
        
        // Ensure we have enough to repay loan + profit
        uint256 amountOwed = amount + premium;
        require(IERC20(asset).balanceOf(address(this)) >= amountOwed, "Insufficient funds for repayment");
        require(profit >= arbParams.minProfit, "Insufficient profit");
        
        // Approve Aave to pull the debt + premium
        IERC20(asset).approve(address(POOL), amountOwed);
        
        // Transfer profit to owner
        if (profit > 0) {
            IERC20(asset).transfer(owner, profit);
        }
        
        emit ArbitrageExecuted(arbParams.tokenIn, arbParams.tokenOut, amount, profit);
        
        return true;
    }
    
    /**
     * @dev Internal arbitrage execution logic
     */
    function _executeArbitrage(ArbitrageParams memory params) internal returns (uint256 profit) {
        uint256 startBalance = IERC20(params.tokenIn).balanceOf(address(this));
        
        // Step 1: Buy tokenOut with tokenIn on first DEX
        uint256 tokenOutAmount = _executeTrade(
            params.tokenIn,
            params.tokenOut,
            params.amountIn,
            params.buyRouter,
            params.buyFee
        );
        
        // Step 2: Sell tokenOut for tokenIn on second DEX
        uint256 tokenInReceived = _executeTrade(
            params.tokenOut,
            params.tokenIn,
            tokenOutAmount,
            params.sellRouter,
            params.sellFee
        );
        
        uint256 endBalance = IERC20(params.tokenIn).balanceOf(address(this));
        
        // Calculate profit (should be positive for profitable arbitrage)
        require(endBalance > startBalance, "Trade resulted in loss");
        profit = endBalance - startBalance;
    }
    
    /**
     * @dev Execute trade on specified router
     */
    function _executeTrade(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        address router,
        uint24 fee
    ) internal returns (uint256 amountOut) {
        IERC20(tokenIn).approve(router, amountIn);
        
        if (fee == 0) {
            // V2 Router
            address[] memory path = new address[](2);
            path[0] = tokenIn;
            path[1] = tokenOut;
            
            uint256[] memory amounts = IUniswapV2Router(router).swapExactTokensForTokens(
                amountIn,
                0, // Accept any amount (we've already verified profitability)
                path,
                address(this),
                block.timestamp + 300
            );
            
            amountOut = amounts[1];
        } else {
            // V3 Router
            IUniswapV3Router.ExactInputSingleParams memory swapParams = IUniswapV3Router.ExactInputSingleParams({
                tokenIn: tokenIn,
                tokenOut: tokenOut,
                fee: fee,
                recipient: address(this),
                deadline: block.timestamp + 300,
                amountIn: amountIn,
                amountOutMinimum: 0,
                sqrtPriceLimitX96: 0
            });
            
            amountOut = IUniswapV3Router(router).exactInputSingle(swapParams);
        }
    }
    
    /**
     * @dev Preview arbitrage profit without executing
     */
    function previewArbitrage(ArbitrageParams calldata params) external view returns (uint256 expectedProfit) {
        // Get amount out from first trade
        uint256 intermediateAmount;
        
        if (params.buyFee == 0) {
            // V2 Router
            address[] memory path = new address[](2);
            path[0] = params.tokenIn;
            path[1] = params.tokenOut;
            
            uint256[] memory amounts = IUniswapV2Router(params.buyRouter).getAmountsOut(params.amountIn, path);
            intermediateAmount = amounts[1];
        } else {
            // For V3, would need to call quoter contract (simplified here)
            intermediateAmount = params.amountIn; // Placeholder
        }
        
        // Get amount out from second trade
        uint256 finalAmount;
        
        if (params.sellFee == 0) {
            // V2 Router
            address[] memory path = new address[](2);
            path[0] = params.tokenOut;
            path[1] = params.tokenIn;
            
            uint256[] memory amounts = IUniswapV2Router(params.sellRouter).getAmountsOut(intermediateAmount, path);
            finalAmount = amounts[1];
        } else {
            // For V3, would need quoter
            finalAmount = intermediateAmount; // Placeholder
        }
        
        // Calculate profit minus flash loan fee (0.09% on Aave V3)
        uint256 flashLoanFee = (params.amountIn * 9) / 10000;
        
        if (finalAmount > params.amountIn + flashLoanFee) {
            expectedProfit = finalAmount - params.amountIn - flashLoanFee;
        } else {
            expectedProfit = 0;
        }
    }
    
    /**
     * @dev Emergency function to recover stuck tokens
     */
    function emergencyWithdraw(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
    
    /**
     * @dev Update owner
     */
    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "Invalid address");
        owner = newOwner;
    }
}