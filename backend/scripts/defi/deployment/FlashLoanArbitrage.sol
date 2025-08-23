// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

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

contract FlashLoanArbitrage is IFlashLoanReceiver {
    address private owner;
    
    // Aave V3 on Polygon
    IPoolAddressesProvider constant ADDRESSES_PROVIDER = 
        IPoolAddressesProvider(0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb);
    
    // Token addresses
    address constant WPOL = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address constant USDC_OLD = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address constant USDC_NEW = 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359;
    
    // Router addresses
    address constant SUSHISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function executeArbitrage(uint256 flashAmount) external onlyOwner {
        address[] memory assets = new address[](1);
        assets[0] = USDC_OLD;
        
        uint256[] memory amounts = new uint256[](1);
        amounts[0] = flashAmount;
        
        uint256[] memory modes = new uint256[](1);
        modes[0] = 0; // 0 = no debt, 1 = stable debt, 2 = variable debt
        
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
        // This is called by Aave after giving us the flash loan
        require(msg.sender == ADDRESSES_PROVIDER.getPool(), "Invalid caller");
        require(initiator == address(this), "Invalid initiator");
        
        uint256 amountBorrowed = amounts[0];
        uint256 totalDebt = amountBorrowed + premiums[0];
        
        // Execute arbitrage
        // 1. Approve router to spend USDC_OLD
        IERC20(USDC_OLD).approve(SUSHISWAP_ROUTER, amountBorrowed);
        
        // 2. Swap USDC_OLD -> WPOL (buy cheap)
        address[] memory path1 = new address[](2);
        path1[0] = USDC_OLD;
        path1[1] = WPOL;
        
        uint256[] memory amounts1 = IUniswapV2Router(SUSHISWAP_ROUTER).swapExactTokensForTokens(
            amountBorrowed,
            0,
            path1,
            address(this),
            block.timestamp + 300
        );
        
        uint256 wpolReceived = amounts1[1];
        
        // 3. Approve router to spend WPOL
        IERC20(WPOL).approve(SUSHISWAP_ROUTER, wpolReceived);
        
        // 4. Swap WPOL -> USDC_NEW (sell expensive)
        address[] memory path2 = new address[](2);
        path2[0] = WPOL;
        path2[1] = USDC_NEW;
        
        uint256[] memory amounts2 = IUniswapV2Router(SUSHISWAP_ROUTER).swapExactTokensForTokens(
            wpolReceived,
            0,
            path2,
            address(this),
            block.timestamp + 300
        );
        
        uint256 usdcNewReceived = amounts2[1];
        
        // 5. Convert USDC_NEW back to USDC_OLD to repay loan
        // This assumes there's a pool or you can do 1:1 conversion
        // In reality, you might need to find a USDC_NEW/USDC_OLD pool
        
        // For now, let's assume we can convert 1:1 (you'd need to implement this)
        // If USDC_NEW and USDC_OLD trade at parity, profit is:
        // usdcNewReceived - totalDebt
        
        // 6. Approve Aave to pull back the loan + fee
        IERC20(USDC_OLD).approve(ADDRESSES_PROVIDER.getPool(), totalDebt);
        
        // 7. Send profit to owner (if using USDC_NEW)
        uint256 profit = IERC20(USDC_NEW).balanceOf(address(this));
        if (profit > 0) {
            IERC20(USDC_NEW).transfer(owner, profit);
        }
        
        return true;
    }
    
    function checkProfitability(uint256 flashAmount) external view returns (
        uint256 expectedWpol,
        uint256 expectedUsdcNew,
        uint256 flashFee,
        uint256 netProfit
    ) {
        // Calculate flash loan fee (0.05% on Aave V3)
        flashFee = (flashAmount * 5) / 10000;
        
        // Get expected output from first swap
        address[] memory path1 = new address[](2);
        path1[0] = USDC_OLD;
        path1[1] = WPOL;
        uint256[] memory amounts1 = IUniswapV2Router(SUSHISWAP_ROUTER).getAmountsOut(flashAmount, path1);
        expectedWpol = amounts1[1];
        
        // Get expected output from second swap
        address[] memory path2 = new address[](2);
        path2[0] = WPOL;
        path2[1] = USDC_NEW;
        uint256[] memory amounts2 = IUniswapV2Router(SUSHISWAP_ROUTER).getAmountsOut(expectedWpol, path2);
        expectedUsdcNew = amounts2[1];
        
        // Calculate net profit (assuming 1:1 USDC conversion)
        uint256 totalDebt = flashAmount + flashFee;
        if (expectedUsdcNew > totalDebt) {
            netProfit = expectedUsdcNew - totalDebt;
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