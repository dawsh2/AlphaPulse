// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface IFlashLoanRecipient {
    function receiveFlashLoan(
        IERC20[] memory tokens,
        uint256[] memory amounts,
        uint256[] memory feeAmounts,
        bytes memory userData
    ) external;
}

interface IBalancerVault {
    function flashLoan(
        IFlashLoanRecipient recipient,
        IERC20[] memory tokens,
        uint256[] memory amounts,
        bytes memory userData
    ) external;
}

interface IUniswapV2Router {
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);
}

interface ISwapRouter {
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

contract FlashArbitrage is IFlashLoanRecipient {
    address private owner;
    IBalancerVault constant VAULT = IBalancerVault(0xBA12222222228d8Ba445958a75a0704d566BF2C8);
    
    // Routers
    address constant QUICKSWAP = 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff;
    address constant UNISWAP_V3 = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
    address constant SUSHISWAP = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    
    // Tokens
    address constant USDC = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant WPOL = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    
    struct ArbParams {
        address buyPool;
        address sellPool;
        uint8 buyRouter;  // 0=QuickSwap, 1=UniV3, 2=SushiSwap
        uint8 sellRouter;
        uint24 buyFee;    // For V3 pools
        uint24 sellFee;   // For V3 pools
        uint256 amount;
        uint256 minProfit;
    }
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    // Main entry point - no capital needed!
    function executeArbitrage(ArbParams calldata params) external onlyOwner {
        // Request flash loan from Balancer
        IERC20[] memory tokens = new IERC20[](1);
        tokens[0] = IERC20(USDC);
        
        uint256[] memory amounts = new uint256[](1);
        amounts[0] = params.amount;
        
        // Encode params for the callback
        bytes memory userData = abi.encode(params);
        
        // This will call receiveFlashLoan below
        VAULT.flashLoan(this, tokens, amounts, userData);
    }
    
    // Callback from Balancer
    function receiveFlashLoan(
        IERC20[] memory tokens,
        uint256[] memory amounts,
        uint256[] memory feeAmounts,
        bytes memory userData
    ) external override {
        require(msg.sender == address(VAULT), "Not vault");
        
        ArbParams memory params = abi.decode(userData, (ArbParams));
        uint256 amountBorrowed = amounts[0];
        uint256 fee = feeAmounts[0];
        
        // Execute arbitrage
        uint256 finalAmount = _performArbitrage(params, amountBorrowed);
        
        // Ensure profit after fees
        uint256 totalOwed = amountBorrowed + fee;
        require(finalAmount >= totalOwed + params.minProfit, "Not profitable");
        
        // Repay flash loan
        IERC20(USDC).transfer(address(VAULT), totalOwed);
        
        // Send profit to owner
        uint256 profit = finalAmount - totalOwed;
        if (profit > 0) {
            IERC20(USDC).transfer(owner, profit);
        }
    }
    
    function _performArbitrage(ArbParams memory params, uint256 amount) private returns (uint256) {
        // Buy WPOL with USDC
        uint256 wpolAmount = _executeSwap(
            USDC,
            WPOL,
            amount,
            params.buyRouter,
            params.buyFee
        );
        
        // Sell WPOL for USDC
        uint256 usdcAmount = _executeSwap(
            WPOL,
            USDC,
            wpolAmount,
            params.sellRouter,
            params.sellFee
        );
        
        return usdcAmount;
    }
    
    function _executeSwap(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        uint8 routerType,
        uint24 fee
    ) private returns (uint256) {
        if (routerType == 1) {
            // UniswapV3
            IERC20(tokenIn).approve(UNISWAP_V3, amountIn);
            
            ISwapRouter.ExactInputSingleParams memory params = ISwapRouter.ExactInputSingleParams({
                tokenIn: tokenIn,
                tokenOut: tokenOut,
                fee: fee,
                recipient: address(this),
                deadline: block.timestamp,
                amountIn: amountIn,
                amountOutMinimum: 0,
                sqrtPriceLimitX96: 0
            });
            
            return ISwapRouter(UNISWAP_V3).exactInputSingle(params);
        } else {
            // V2 style (QuickSwap or SushiSwap)
            address router = routerType == 0 ? QUICKSWAP : SUSHISWAP;
            IERC20(tokenIn).approve(router, amountIn);
            
            address[] memory path = new address[](2);
            path[0] = tokenIn;
            path[1] = tokenOut;
            
            uint[] memory amounts = IUniswapV2Router(router).swapExactTokensForTokens(
                amountIn,
                0,
                path,
                address(this),
                block.timestamp
            );
            
            return amounts[1];
        }
    }
    
    // Emergency withdraw
    function withdraw(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
    
    // Check profitability without executing
    function checkProfitability(ArbParams calldata params) external view returns (int256) {
        // This would need to simulate the swaps
        // For now, return 0 (implement off-chain)
        return 0;
    }
}