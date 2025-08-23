// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface ISolidlyPair {
    function getReserves() external view returns (uint256 reserve0, uint256 reserve1, uint256 blockTimestampLast);
    function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes calldata data) external;
    function stable() external view returns (bool);
    function getAmountOut(uint256 amountIn, address tokenIn) external view returns (uint256);
    function token0() external view returns (address);
    function token1() external view returns (address);
}

interface IUniswapV2Pair {
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes calldata data) external;
}

contract DystopiaArbitrageFixed {
    address private owner;
    
    address constant WPOL = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant USDC_OLD = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant USDC_NEW = 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359;
    
    address constant DYSTOPIA_POOL = 0x380615F37993B5A96adF3D443b6E0Ac50a211998;
    address constant QUICKSWAP_POOL = 0x6D9e8dbB2779853db00418D4DcF96F3987CFC9D2;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function executeArbitrage() external onlyOwner {
        // Get USDC_OLD balance
        uint256 usdcOldBalance = IERC20(USDC_OLD).balanceOf(address(this));
        require(usdcOldBalance > 0, "No USDC_OLD");
        
        // Step 1: Swap USDC_OLD for WPOL on Dystopia (stable pool)
        IERC20(USDC_OLD).transfer(DYSTOPIA_POOL, usdcOldBalance);
        
        // Use Dystopia's getAmountOut for accurate calculation
        ISolidlyPair dystopia = ISolidlyPair(DYSTOPIA_POOL);
        uint256 wpolOut = dystopia.getAmountOut(usdcOldBalance, USDC_OLD);
        
        // Execute swap - Dystopia uses Solidly interface
        dystopia.swap(wpolOut, 0, address(this), "");
        
        // Step 2: Swap WPOL for USDC_NEW on QuickSwap
        IERC20(WPOL).transfer(QUICKSWAP_POOL, wpolOut);
        
        // Calculate output for QuickSwap (standard V2)
        IUniswapV2Pair quickswap = IUniswapV2Pair(QUICKSWAP_POOL);
        (uint112 r0, uint112 r1,) = quickswap.getReserves();
        uint256 usdcNewOut = (wpolOut * 997 * r1) / (r0 * 1000 + wpolOut * 997);
        
        // Execute swap - send USDC_NEW directly to owner
        quickswap.swap(0, usdcNewOut, owner, "");
    }
    
    function checkProfitability(uint256 amount) external view returns (
        uint256 expectedWpol,
        uint256 expectedUsdcNew,
        uint256 profit
    ) {
        // Get WPOL output from Dystopia
        ISolidlyPair dystopia = ISolidlyPair(DYSTOPIA_POOL);
        expectedWpol = dystopia.getAmountOut(amount, USDC_OLD);
        
        // Calculate USDC_NEW output from QuickSwap
        IUniswapV2Pair quickswap = IUniswapV2Pair(QUICKSWAP_POOL);
        (uint112 r0, uint112 r1,) = quickswap.getReserves();
        expectedUsdcNew = (expectedWpol * 997 * r1) / (r0 * 1000 + expectedWpol * 997);
        
        profit = expectedUsdcNew > amount ? expectedUsdcNew - amount : 0;
    }
    
    function withdraw(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
}