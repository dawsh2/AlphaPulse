// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface IDystopiaRouter {
    struct route {
        address from;
        address to;
        bool stable;
    }
    
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        route[] calldata routes,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);
    
    function getAmountsOut(uint amountIn, route[] calldata routes) 
        external view returns (uint[] memory amounts);
}

interface IQuickSwapRouter {
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);
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

contract DystopiaArbitrage is IFlashLoanReceiver {
    address private owner;
    
    // Aave V3 on Polygon
    IPoolAddressesProvider constant ADDRESSES_PROVIDER = 
        IPoolAddressesProvider(0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb);
    
    // Routers
    IDystopiaRouter constant DYSTOPIA_ROUTER = IDystopiaRouter(0xbE75Dd16D029c6B32B7aD57A0FD9C1c20Dd2862e);
    IQuickSwapRouter constant QUICKSWAP_ROUTER = IQuickSwapRouter(0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff);
    
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
        
        // Step 1: Buy WPOL with USDC_OLD using Dystopia (WPOL is cheap here)
        IERC20(USDC_OLD).approve(address(DYSTOPIA_ROUTER), amountBorrowed);
        
        IDystopiaRouter.route[] memory dystopiaRoute = new IDystopiaRouter.route[](1);
        dystopiaRoute[0] = IDystopiaRouter.route({
            from: USDC_OLD,
            to: WPOL,
            stable: false  // Not a stable pair
        });
        
        uint[] memory amounts1 = DYSTOPIA_ROUTER.swapExactTokensForTokens(
            amountBorrowed,
            0, // Accept any amount
            dystopiaRoute,
            address(this),
            block.timestamp + 300
        );
        
        uint256 wpolReceived = amounts1[amounts1.length - 1];
        
        // Step 2: Sell WPOL for USDC_NEW using QuickSwap (WPOL is expensive here)
        IERC20(WPOL).approve(address(QUICKSWAP_ROUTER), wpolReceived);
        
        address[] memory quickswapPath = new address[](2);
        quickswapPath[0] = WPOL;
        quickswapPath[1] = USDC_NEW;
        
        uint[] memory amounts2 = QUICKSWAP_ROUTER.swapExactTokensForTokens(
            wpolReceived,
            0, // Accept any amount
            quickswapPath,
            address(this),
            block.timestamp + 300
        );
        
        uint256 usdcNewReceived = amounts2[amounts2.length - 1];
        
        // For this to work, we need USDC_NEW = USDC_OLD value
        // In production, you'd swap USDC_NEW -> USDC_OLD here
        
        // Approve Aave to pull back the loan
        IERC20(USDC_OLD).approve(ADDRESSES_PROVIDER.getPool(), totalDebt);
        
        // Send profit to owner (USDC_NEW)
        uint256 profit = IERC20(USDC_NEW).balanceOf(address(this));
        if (profit > 0) {
            IERC20(USDC_NEW).transfer(owner, profit);
        }
        
        return true;
    }
    
    function checkProfitability(uint256 amount) external view returns (
        uint256 expectedWpol,
        uint256 expectedUsdcNew,
        uint256 flashFee,
        uint256 netProfit
    ) {
        // Get expected output from Dystopia
        IDystopiaRouter.route[] memory dystopiaRoute = new IDystopiaRouter.route[](1);
        dystopiaRoute[0] = IDystopiaRouter.route({
            from: USDC_OLD,
            to: WPOL,
            stable: false
        });
        
        try DYSTOPIA_ROUTER.getAmountsOut(amount, dystopiaRoute) returns (uint[] memory amounts1) {
            expectedWpol = amounts1[amounts1.length - 1];
            
            // Estimate QuickSwap output (this is approximate)
            // In production, query QuickSwap router for exact amount
            expectedUsdcNew = expectedWpol * 240 / 1000000; // Rough estimate
            
        } catch {
            expectedWpol = 0;
            expectedUsdcNew = 0;
        }
        
        flashFee = (amount * 5) / 10000; // 0.05%
        uint256 totalCost = amount + flashFee;
        
        if (expectedUsdcNew > totalCost) {
            netProfit = expectedUsdcNew - totalCost;
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