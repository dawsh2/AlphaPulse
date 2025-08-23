// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface IUniswapV2Pair {
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    function swap(uint amount0Out, uint amount1Out, address to, bytes calldata data) external;
    function token0() external view returns (address);
    function token1() external view returns (address);
}

interface IFlashLoanReceiver {
    function executeOperation(
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata premiums,
        address initiator,
        bytes calldata params
    ) external returns (bool);
}

interface IPoolAddressesProvider {
    function getPool() external view returns (address);
}

interface IPool {
    function flashLoan(
        address receiverAddress,
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata modes,
        address onBehalfOf,
        bytes calldata params,
        uint16 referralCode
    ) external;
}

contract DirectPoolArbitrage is IFlashLoanReceiver {
    address private owner;
    
    // Aave V3 on Polygon
    IPoolAddressesProvider constant ADDRESSES_PROVIDER = 
        IPoolAddressesProvider(0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb);
    
    // Token addresses
    address constant WPOL = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant USDC_OLD = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant USDC_NEW = 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    // Direct swap with specific V2 pool
    function swapInPool(address pool, address tokenIn, uint256 amountIn) internal returns (uint256) {
        IUniswapV2Pair pair = IUniswapV2Pair(pool);
        
        // Transfer tokens to pool
        IERC20(tokenIn).transfer(pool, amountIn);
        
        // Get reserves
        (uint112 reserve0, uint112 reserve1,) = pair.getReserves();
        
        // Determine direction
        address token0 = pair.token0();
        bool isToken0In = tokenIn == token0;
        
        uint256 amountOut;
        if (isToken0In) {
            // Selling token0 for token1
            amountOut = getAmountOut(amountIn, reserve0, reserve1);
            pair.swap(0, amountOut, address(this), "");
        } else {
            // Selling token1 for token0
            amountOut = getAmountOut(amountIn, reserve1, reserve0);
            pair.swap(amountOut, 0, address(this), "");
        }
        
        return amountOut;
    }
    
    // Calculate output amount for V2 swap
    function getAmountOut(uint256 amountIn, uint256 reserveIn, uint256 reserveOut) internal pure returns (uint256) {
        uint256 amountInWithFee = amountIn * 997;
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        return numerator / denominator;
    }
    
    function executeArbitrageWithPools(
        uint256 flashAmount,
        address buyPool,
        address sellPool
    ) external onlyOwner {
        // Store pool addresses for use in callback
        bytes memory params = abi.encode(buyPool, sellPool);
        
        address[] memory assets = new address[](1);
        assets[0] = USDC_OLD;
        
        uint256[] memory amounts = new uint256[](1);
        amounts[0] = flashAmount;
        
        uint256[] memory modes = new uint256[](1);
        modes[0] = 0; // No debt
        
        IPool(ADDRESSES_PROVIDER.getPool()).flashLoan(
            address(this),
            assets,
            amounts,
            modes,
            address(this),
            params,
            0
        );
    }
    
    function executeOperation(
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata premiums,
        address initiator,
        bytes calldata params
    ) external override returns (bool) {
        require(msg.sender == ADDRESSES_PROVIDER.getPool(), "Invalid caller");
        require(initiator == address(this), "Invalid initiator");
        
        // Decode pool addresses
        (address buyPool, address sellPool) = abi.decode(params, (address, address));
        
        uint256 amountBorrowed = amounts[0];
        uint256 totalDebt = amountBorrowed + premiums[0];
        
        // Step 1: Buy WPOL with USDC_OLD in buyPool (WPOL is cheap here)
        uint256 wpolReceived = swapInPool(buyPool, USDC_OLD, amountBorrowed);
        
        // Step 2: Sell WPOL for USDC_NEW in sellPool (WPOL is expensive here)
        uint256 usdcNewReceived = swapInPool(sellPool, WPOL, wpolReceived);
        
        // At this point we have USDC_NEW but need USDC_OLD to repay
        // If profitable, usdcNewReceived > totalDebt
        
        // For now, assuming 1:1 conversion or having a conversion method
        // In production, you'd need to swap USDC_NEW -> USDC_OLD
        
        // Approve Aave to pull back the loan
        IERC20(USDC_OLD).approve(ADDRESSES_PROVIDER.getPool(), totalDebt);
        
        // Send profit to owner
        uint256 profit = IERC20(USDC_NEW).balanceOf(address(this));
        if (profit > 0) {
            IERC20(USDC_NEW).transfer(owner, profit);
        }
        
        return true;
    }
    
    // Check profitability before executing
    function checkProfitability(
        address buyPool,
        address sellPool,
        uint256 amount
    ) external view returns (uint256 expectedProfit) {
        IUniswapV2Pair buyPair = IUniswapV2Pair(buyPool);
        IUniswapV2Pair sellPair = IUniswapV2Pair(sellPool);
        
        // Get reserves
        (uint112 buyR0, uint112 buyR1,) = buyPair.getReserves();
        (uint112 sellR0, uint112 sellR1,) = sellPair.getReserves();
        
        // Assuming token0 is WPOL, token1 is USDC variant
        // Calculate: USDC -> WPOL -> USDC.e
        
        // Step 1: USDC -> WPOL in buy pool
        uint256 wpolOut = getAmountOut(amount, buyR1, buyR0);
        
        // Step 2: WPOL -> USDC.e in sell pool  
        uint256 usdcOut = getAmountOut(wpolOut, sellR0, sellR1);
        
        // Flash loan fee
        uint256 flashFee = (amount * 5) / 10000; // 0.05%
        uint256 totalCost = amount + flashFee;
        
        if (usdcOut > totalCost) {
            expectedProfit = usdcOut - totalCost;
        } else {
            expectedProfit = 0;
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