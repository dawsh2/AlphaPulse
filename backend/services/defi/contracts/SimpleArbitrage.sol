// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

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
 * @title SimpleArbitrage
 * @dev Basic arbitrage contract for testing and small-scale execution
 * Uses existing wallet balance instead of flash loans
 */
contract SimpleArbitrage is ReentrancyGuard {
    address private owner;
    
    // DEX Routers
    address constant QUICKSWAP_ROUTER = 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff;
    address constant SUSHISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    address constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
    
    // Common tokens
    address constant WMATIC = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant USDC = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant USDT = 0xc2132D05D31c914a87C6611C10748AEb04B58e8F;
    
    event ArbitrageExecuted(
        address indexed tokenIn,
        address indexed tokenOut,
        uint256 amountIn,
        uint256 profit,
        uint256 gasUsed
    );
    
    event TradeFailed(
        address indexed tokenIn,
        address indexed tokenOut,
        uint256 amountIn,
        string reason
    );
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    /**
     * @dev Execute cross-DEX arbitrage using wallet balance
     * @param tokenIn Input token address
     * @param tokenOut Intermediate token address
     * @param amountIn Amount of tokenIn to use
     * @param buyRouter Router to buy tokenOut
     * @param sellRouter Router to sell tokenOut back to tokenIn
     * @param buyFee V3 fee tier for buy (0 for V2)
     * @param sellFee V3 fee tier for sell (0 for V2)
     */
    function executeArbitrage(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        address buyRouter,
        address sellRouter,
        uint24 buyFee,
        uint24 sellFee
    ) external onlyOwner nonReentrant returns (uint256 profit) {
        uint256 startGas = gasleft();
        
        require(amountIn > 0, "Invalid amount");
        require(tokenIn != tokenOut, "Same token");
        
        // Transfer tokens from owner
        IERC20(tokenIn).transferFrom(msg.sender, address(this), amountIn);
        
        uint256 startBalance = IERC20(tokenIn).balanceOf(address(this));
        
        try this.internalArbitrage(
            tokenIn, 
            tokenOut, 
            amountIn, 
            buyRouter, 
            sellRouter, 
            buyFee, 
            sellFee
        ) returns (uint256 finalBalance) {
            
            if (finalBalance > startBalance) {
                profit = finalBalance - startBalance;
            }
            
            // Transfer all tokens back to owner
            IERC20(tokenIn).transfer(msg.sender, finalBalance);
            
            // Also transfer any remaining tokenOut
            uint256 tokenOutBalance = IERC20(tokenOut).balanceOf(address(this));
            if (tokenOutBalance > 0) {
                IERC20(tokenOut).transfer(msg.sender, tokenOutBalance);
            }
            
            uint256 gasUsed = startGas - gasleft();
            emit ArbitrageExecuted(tokenIn, tokenOut, amountIn, profit, gasUsed);
            
        } catch Error(string memory reason) {
            // Return remaining tokens on failure
            uint256 remainingBalance = IERC20(tokenIn).balanceOf(address(this));
            if (remainingBalance > 0) {
                IERC20(tokenIn).transfer(msg.sender, remainingBalance);
            }
            
            uint256 tokenOutBalance = IERC20(tokenOut).balanceOf(address(this));
            if (tokenOutBalance > 0) {
                IERC20(tokenOut).transfer(msg.sender, tokenOutBalance);
            }
            
            emit TradeFailed(tokenIn, tokenOut, amountIn, reason);
        }
    }
    
    /**
     * @dev Internal arbitrage logic (external for try/catch)
     */
    function internalArbitrage(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        address buyRouter,
        address sellRouter,
        uint24 buyFee,
        uint24 sellFee
    ) external returns (uint256 finalBalance) {
        require(msg.sender == address(this), "Internal only");
        
        // Step 1: Buy tokenOut with tokenIn
        uint256 tokenOutAmount = _executeTrade(
            tokenIn,
            tokenOut,
            amountIn,
            buyRouter,
            buyFee
        );
        
        require(tokenOutAmount > 0, "Buy trade failed");
        
        // Step 2: Sell tokenOut for tokenIn
        uint256 tokenInReceived = _executeTrade(
            tokenOut,
            tokenIn,
            tokenOutAmount,
            sellRouter,
            sellFee
        );
        
        require(tokenInReceived > 0, "Sell trade failed");
        
        finalBalance = IERC20(tokenIn).balanceOf(address(this));
    }
    
    /**
     * @dev Execute single trade on specified router
     */
    function _executeTrade(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        address router,
        uint24 fee
    ) internal returns (uint256 amountOut) {
        // Approve router to spend tokens
        IERC20(tokenIn).approve(router, amountIn);
        
        if (fee == 0) {
            // V2 Router
            address[] memory path = new address[](2);
            path[0] = tokenIn;
            path[1] = tokenOut;
            
            uint256[] memory amounts = IUniswapV2Router(router).swapExactTokensForTokens(
                amountIn,
                0, // Accept any amount (slippage handled by caller)
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
        
        require(amountOut > 0, "Trade returned zero");
    }
    
    /**
     * @dev Simple round-trip arbitrage for testing
     */
    function executeSimpleArbitrage(
        address tokenIn,
        address tokenOut,
        uint256 amountIn
    ) external onlyOwner nonReentrant returns (uint256 profit) {
        return executeArbitrage(
            tokenIn,
            tokenOut,
            amountIn,
            QUICKSWAP_ROUTER,
            QUICKSWAP_ROUTER,
            0, // V2
            0  // V2
        );
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
     * @dev Transfer ownership
     */
    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "Invalid address");
        owner = newOwner;
    }
    
    /**
     * @dev Get owner address
     */
    function getOwner() external view returns (address) {
        return owner;
    }
}