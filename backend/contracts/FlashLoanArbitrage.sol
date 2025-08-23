// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IFlashLoanReceiver {
    function executeOperation(
        address asset,
        uint256 amount,
        uint256 premium,
        address initiator,
        bytes calldata params
    ) external returns (bool);
}

interface IPool {
    function flashLoanSimple(
        address receiverAddress,
        address asset,
        uint256 amount,
        bytes calldata params,
        uint16 referralCode
    ) external;
}

interface IERC20 {
    function approve(address spender, uint256 amount) external returns (bool);
    function transfer(address to, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
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

contract FlashLoanArbitrage is IFlashLoanReceiver {
    address constant AAVE_POOL = 0x794a61358D6845594F94dc1DB02A252b5b4814aD;
    address constant USDC = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant WMATIC = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    
    address immutable owner;
    
    struct ArbParams {
        address buyRouter;
        address sellRouter;
        address tokenA;
        address tokenB;
        uint256 minProfit;
    }
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function executeArbitrage(
        uint256 amount,
        address buyRouter,
        address sellRouter,
        address tokenB,
        uint256 minProfit
    ) external onlyOwner {
        // Encode parameters for the flash loan callback
        bytes memory params = abi.encode(ArbParams({
            buyRouter: buyRouter,
            sellRouter: sellRouter,
            tokenA: USDC,
            tokenB: tokenB,
            minProfit: minProfit
        }));
        
        // Request flash loan from Aave
        IPool(AAVE_POOL).flashLoanSimple(
            address(this),
            USDC,
            amount,
            params,
            0
        );
    }
    
    function executeOperation(
        address asset,
        uint256 amount,
        uint256 premium,
        address initiator,
        bytes calldata params
    ) external override returns (bool) {
        require(msg.sender == AAVE_POOL, "Invalid caller");
        require(initiator == address(this), "Invalid initiator");
        
        // Decode parameters
        ArbParams memory arbParams = abi.decode(params, (ArbParams));
        
        // 1. Approve and swap USDC for tokenB on first DEX
        IERC20(USDC).approve(arbParams.buyRouter, amount);
        
        address[] memory buyPath = new address[](2);
        buyPath[0] = USDC;
        buyPath[1] = arbParams.tokenB;
        
        uint[] memory amounts = IRouter(arbParams.buyRouter).swapExactTokensForTokens(
            amount,
            0, // Calculate off-chain
            buyPath,
            address(this),
            block.timestamp + 300
        );
        
        uint256 tokenBReceived = amounts[amounts.length - 1];
        
        // 2. Approve and swap tokenB back to USDC on second DEX
        IERC20(arbParams.tokenB).approve(arbParams.sellRouter, tokenBReceived);
        
        address[] memory sellPath = new address[](2);
        sellPath[0] = arbParams.tokenB;
        sellPath[1] = USDC;
        
        amounts = IRouter(arbParams.sellRouter).swapExactTokensForTokens(
            tokenBReceived,
            amount + premium + arbParams.minProfit, // Must cover loan + fee + profit
            sellPath,
            address(this),
            block.timestamp + 300
        );
        
        // 3. Approve Aave to take back the loan + fee
        uint256 totalDebt = amount + premium;
        IERC20(USDC).approve(AAVE_POOL, totalDebt);
        
        // 4. Transfer profit to owner
        uint256 profit = IERC20(USDC).balanceOf(address(this)) - totalDebt;
        if (profit > 0) {
            IERC20(USDC).transfer(owner, profit);
        }
        
        return true;
    }
    
    // Emergency withdrawal function
    function withdraw(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
    
    // Receive function for MATIC
    receive() external payable {}
}