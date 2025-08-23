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

contract SimpleDirectArbitrage {
    address private owner;
    
    // Token addresses
    address constant WPOL = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant USDC_OLD = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant USDC_NEW = 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359;
    
    // Pools
    address constant BUY_POOL = 0x380615f37993b5a96adf3d443b6e0ac50a211998;  // Dystopia WPOL/USDC_OLD
    address constant SELL_POOL = 0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2; // QuickSwap WPOL/USDC_NEW
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function getAmountOut(uint256 amountIn, uint256 reserveIn, uint256 reserveOut) internal pure returns (uint256) {
        uint256 amountInWithFee = amountIn * 997;
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        return numerator / denominator;
    }
    
    // Execute arbitrage without flash loan - owner must have USDC_OLD
    function executeArbitrage(uint256 usdcAmount) external onlyOwner {
        // Transfer USDC_OLD from owner
        IERC20(USDC_OLD).transfer(BUY_POOL, usdcAmount);
        
        // Get pool reserves to calculate output
        IUniswapV2Pair buyPair = IUniswapV2Pair(BUY_POOL);
        (uint112 buyR0, uint112 buyR1,) = buyPair.getReserves();
        
        // Buy pool: token0=WPOL, token1=USDC_OLD
        uint256 wpolOut = getAmountOut(usdcAmount, buyR1, buyR0);
        
        // Execute swap: USDC_OLD -> WPOL
        buyPair.swap(wpolOut, 0, address(this), "");
        
        // Now sell WPOL for USDC_NEW
        IERC20(WPOL).transfer(SELL_POOL, wpolOut);
        
        IUniswapV2Pair sellPair = IUniswapV2Pair(SELL_POOL);
        (uint112 sellR0, uint112 sellR1,) = sellPair.getReserves();
        
        // Sell pool: token0=WPOL, token1=USDC_NEW
        uint256 usdcNewOut = getAmountOut(wpolOut, sellR0, sellR1);
        
        // Execute swap: WPOL -> USDC_NEW
        sellPair.swap(0, usdcNewOut, address(this), "");
        
        // Send USDC_NEW to owner
        IERC20(USDC_NEW).transfer(owner, usdcNewOut);
    }
    
    // Check profitability
    function checkProfitability(uint256 amount) external view returns (
        uint256 expectedWpol,
        uint256 expectedUsdcNew,
        uint256 profit
    ) {
        IUniswapV2Pair buyPair = IUniswapV2Pair(BUY_POOL);
        IUniswapV2Pair sellPair = IUniswapV2Pair(SELL_POOL);
        
        (uint112 buyR0, uint112 buyR1,) = buyPair.getReserves();
        (uint112 sellR0, uint112 sellR1,) = sellPair.getReserves();
        
        // Buy pool: token0=WPOL, token1=USDC_OLD
        expectedWpol = getAmountOut(amount, buyR1, buyR0);
        
        // Sell pool: token0=WPOL, token1=USDC_NEW
        expectedUsdcNew = getAmountOut(expectedWpol, sellR0, sellR1);
        
        if (expectedUsdcNew > amount) {
            profit = expectedUsdcNew - amount;
        } else {
            profit = 0;
        }
    }
    
    // Withdraw any token
    function withdraw(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
    
    receive() external payable {}
}