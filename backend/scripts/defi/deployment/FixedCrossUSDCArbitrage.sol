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

interface IFlashLoanReceiver {
    function executeOperation(
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata premiums,
        address initiator,
        bytes calldata params
    ) external returns (bool);
}

// Curve pool for USDC conversion
interface ICurvePool {
    function exchange(int128 i, int128 j, uint256 dx, uint256 min_dy) external returns (uint256);
}

contract FixedCrossUSDCArbitrage is IFlashLoanReceiver {
    address private owner;
    
    // Aave V3 on Polygon
    IPoolAddressesProvider constant ADDRESSES_PROVIDER = 
        IPoolAddressesProvider(0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb);
    
    // Token addresses
    address constant WPOL = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant USDC_OLD = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant USDC_NEW = 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359;
    
    // Pools
    address constant BUY_POOL = 0x380615f37993b5a96adf3d443b6e0ac50a211998;  // Dystopia
    address constant SELL_POOL = 0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2; // QuickSwap
    
    // Curve USDC pool for conversion (if exists)
    address constant CURVE_USDC_POOL = 0x5ab5C56B9db92Ba45a0B46a207286cD83C15C939; // Curve 2pool USDC/USDC.e
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    // Direct swap with V2-style pool (works for both Dystopia and QuickSwap)
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
    
    function getAmountOut(uint256 amountIn, uint256 reserveIn, uint256 reserveOut) internal pure returns (uint256) {
        uint256 amountInWithFee = amountIn * 997;
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        return numerator / denominator;
    }
    
    function executeArbitrage(uint256 flashAmount) external onlyOwner {
        address[] memory assets = new address[](1);
        assets[0] = USDC_OLD;
        
        uint256[] memory amounts = new uint256[](1);
        amounts[0] = flashAmount;
        
        uint256[] memory modes = new uint256[](1);
        modes[0] = 0; // No debt
        
        bytes memory params = "";
        
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
        
        uint256 amountBorrowed = amounts[0];
        uint256 totalDebt = amountBorrowed + premiums[0];
        
        // Step 1: Buy WPOL with USDC_OLD using Dystopia pool directly (WPOL cheap at $0.082)
        uint256 wpolReceived = swapInPool(BUY_POOL, USDC_OLD, amountBorrowed);
        
        // Step 2: Sell WPOL for USDC_NEW using QuickSwap (WPOL expensive at $0.241)
        uint256 usdcNewReceived = swapInPool(SELL_POOL, WPOL, wpolReceived);
        
        // Step 3: Convert USDC_NEW to USDC_OLD using Curve
        // This is critical - we need USDC_OLD to repay the flash loan
        IERC20(USDC_NEW).approve(CURVE_USDC_POOL, usdcNewReceived);
        
        // Curve pool: index 0 = USDC_OLD, index 1 = USDC_NEW
        // Exchange from USDC_NEW (1) to USDC_OLD (0)
        uint256 usdcOldReceived = ICurvePool(CURVE_USDC_POOL).exchange(
            1,  // from USDC_NEW
            0,  // to USDC_OLD
            usdcNewReceived,
            usdcNewReceived * 995 / 1000  // accept 0.5% slippage
        );
        
        // Step 4: Repay flash loan
        require(usdcOldReceived >= totalDebt, "Unprofitable");
        IERC20(USDC_OLD).approve(ADDRESSES_PROVIDER.getPool(), totalDebt);
        
        // Step 5: Send profit to owner
        uint256 profit = usdcOldReceived - totalDebt;
        if (profit > 0) {
            IERC20(USDC_OLD).transfer(owner, profit);
        }
        
        return true;
    }
    
    // Simpler version without Curve - assumes 1:1 USDC conversion
    function executeArbitrageSimple(uint256 flashAmount) external onlyOwner {
        // For testing - flash loan USDC_NEW instead
        address[] memory assets = new address[](1);
        assets[0] = USDC_NEW;
        
        uint256[] memory amounts = new uint256[](1);
        amounts[0] = flashAmount;
        
        uint256[] memory modes = new uint256[](1);
        modes[0] = 0;
        
        bytes memory params = abi.encode(true); // flag for simple mode
        
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
    
    function executeOperationSimple(
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata premiums,
        bytes calldata params
    ) internal returns (bool) {
        uint256 amountBorrowed = amounts[0];
        uint256 totalDebt = amountBorrowed + premiums[0];
        
        // We have USDC_NEW, need to get USDC_OLD first
        // Step 1: Convert USDC_NEW to USDC_OLD (assuming we can do this somewhere)
        // For now, we'll swap USDC_NEW -> WPOL -> USDC_OLD
        
        // Swap USDC_NEW for WPOL in sell pool (backwards)
        uint256 wpolReceived = swapInPool(SELL_POOL, USDC_NEW, amountBorrowed);
        
        // Swap WPOL for USDC_OLD in buy pool (backwards)
        uint256 usdcOldReceived = swapInPool(BUY_POOL, WPOL, wpolReceived);
        
        // This should be profitable if spreads are right
        require(usdcOldReceived > amountBorrowed, "Not profitable backwards");
        
        // Convert back to USDC_NEW for repayment
        // ... would need conversion here ...
        
        return true;
    }
    
    function checkProfitability(uint256 amount) external view returns (
        uint256 expectedWpol,
        uint256 expectedUsdcNew,
        uint256 expectedUsdcOld,
        uint256 flashFee,
        uint256 netProfit
    ) {
        IUniswapV2Pair buyPair = IUniswapV2Pair(BUY_POOL);
        IUniswapV2Pair sellPair = IUniswapV2Pair(SELL_POOL);
        
        (uint112 buyR0, uint112 buyR1,) = buyPair.getReserves();
        (uint112 sellR0, uint112 sellR1,) = sellPair.getReserves();
        
        // Buy pool: token0=WPOL, token1=USDC_OLD
        // Step 1: USDC_OLD -> WPOL
        expectedWpol = getAmountOut(amount, buyR1, buyR0);
        
        // Sell pool: token0=WPOL, token1=USDC_NEW  
        // Step 2: WPOL -> USDC_NEW
        expectedUsdcNew = getAmountOut(expectedWpol, sellR0, sellR1);
        
        // Assume 1:1 conversion or small fee
        expectedUsdcOld = expectedUsdcNew * 998 / 1000; // 0.2% conversion fee estimate
        
        flashFee = (amount * 5) / 10000; // 0.05%
        uint256 totalCost = amount + flashFee;
        
        if (expectedUsdcOld > totalCost) {
            netProfit = expectedUsdcOld - totalCost;
        } else {
            netProfit = 0;
        }
    }
    
    function withdraw(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
    
    receive() external payable {}
}