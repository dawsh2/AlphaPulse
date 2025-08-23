// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

// Minimal interfaces
interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface IBalancerVault {
    function flashLoan(
        address recipient,
        address[] memory tokens,
        uint256[] memory amounts,
        bytes memory userData
    ) external;
}

interface IRouter {
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);
}

contract FastFlashArb {
    IBalancerVault constant VAULT = IBalancerVault(0xBA12222222228d8Ba445958a75a0704d566BF2C8);
    address immutable owner;
    
    constructor() {
        owner = msg.sender;
    }
    
    // Execute arbitrage with any token pair
    function execute(
        address tokenBorrow,    // Token to borrow (e.g., USDT)
        address tokenMiddle,    // Middle token (e.g., WPOL)
        uint256 borrowAmount,   // Amount to borrow
        address router1,        // Router for first swap
        address router2,        // Router for second swap
        uint256 minProfit      // Minimum profit required
    ) external {
        require(msg.sender == owner, "Not owner");
        
        // Setup flash loan
        address[] memory tokens = new address[](1);
        tokens[0] = tokenBorrow;
        
        uint256[] memory amounts = new uint256[](1);
        amounts[0] = borrowAmount;
        
        // Encode params for callback
        bytes memory params = abi.encode(
            tokenBorrow,
            tokenMiddle,
            router1,
            router2,
            minProfit
        );
        
        // Request flash loan - will call receiveFlashLoan
        VAULT.flashLoan(address(this), tokens, amounts, params);
    }
    
    // Callback from Balancer
    function receiveFlashLoan(
        address[] memory,
        uint256[] memory amounts,
        uint256[] memory feeAmounts,
        bytes memory userData
    ) external {
        require(msg.sender == address(VAULT), "Not Balancer");
        
        // Decode params
        (
            address tokenBorrow,
            address tokenMiddle,
            address router1,
            address router2,
            uint256 minProfit
        ) = abi.decode(userData, (address, address, address, address, uint256));
        
        uint256 borrowedAmount = amounts[0];
        uint256 fee = feeAmounts[0];
        
        // Approve router1
        IERC20(tokenBorrow).approve(router1, borrowedAmount);
        
        // Swap 1: tokenBorrow -> tokenMiddle
        address[] memory path1 = new address[](2);
        path1[0] = tokenBorrow;
        path1[1] = tokenMiddle;
        
        uint[] memory amounts1 = IRouter(router1).swapExactTokensForTokens(
            borrowedAmount,
            0,
            path1,
            address(this),
            block.timestamp
        );
        
        uint256 middleAmount = amounts1[1];
        
        // Approve router2
        IERC20(tokenMiddle).approve(router2, middleAmount);
        
        // Swap 2: tokenMiddle -> tokenBorrow
        address[] memory path2 = new address[](2);
        path2[0] = tokenMiddle;
        path2[1] = tokenBorrow;
        
        uint[] memory amounts2 = IRouter(router2).swapExactTokensForTokens(
            middleAmount,
            0,
            path2,
            address(this),
            block.timestamp
        );
        
        uint256 receivedAmount = amounts2[1];
        
        // Check profit
        uint256 totalOwed = borrowedAmount + fee;
        require(receivedAmount >= totalOwed + minProfit, "Not profitable");
        
        // Repay flash loan
        IERC20(tokenBorrow).transfer(address(VAULT), totalOwed);
        
        // Send profit to owner
        uint256 profit = receivedAmount - totalOwed;
        if (profit > 0) {
            IERC20(tokenBorrow).transfer(owner, profit);
        }
    }
    
    // Withdraw stuck tokens
    function withdraw(address token) external {
        require(msg.sender == owner, "Not owner");
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
}