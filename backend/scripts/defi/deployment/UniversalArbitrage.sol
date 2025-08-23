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
}

contract UniversalArbitrage {
    address private owner;
    
    // Main routers
    address constant QUICKSWAP_ROUTER = 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff;
    address constant SUSHISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    address constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
    address constant DYSTOPIA_ROUTER = 0xbE75Dd16D029c6B32B7aD57A0FD9C1c20Dd2862e;
    
    enum RouterType { V2_QUICKSWAP, V2_SUSHI, V3_UNISWAP, STABLE_DYSTOPIA }
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function executeArbitrage(
        address tokenA,
        address tokenB,
        uint256 amountIn,
        address buyPool,
        RouterType buyRouterType,
        address sellPool,
        RouterType sellRouterType,
        uint24 v3Fee // Only used for V3 pools
    ) external onlyOwner {
        // Approve routers
        address buyRouter = getRouter(buyRouterType);
        address sellRouter = getRouter(sellRouterType);
        
        IERC20(tokenA).approve(buyRouter, amountIn);
        
        // Execute buy swap
        uint256 tokenBReceived = executeSwap(
            tokenA,
            tokenB,
            amountIn,
            buyRouter,
            buyRouterType,
            v3Fee
        );
        
        // Approve for sell swap
        IERC20(tokenB).approve(sellRouter, tokenBReceived);
        
        // Execute sell swap
        uint256 tokenAReceived = executeSwap(
            tokenB,
            tokenA,
            tokenBReceived,
            sellRouter,
            sellRouterType,
            v3Fee
        );
        
        // Profit is tokenAReceived - amountIn
        require(tokenAReceived > amountIn, "No profit");
        
        // Send profit to owner
        IERC20(tokenA).transfer(owner, tokenAReceived);
    }
    
    function executeSwap(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        address router,
        RouterType routerType,
        uint24 v3Fee
    ) internal returns (uint256) {
        if (routerType == RouterType.V2_QUICKSWAP || routerType == RouterType.V2_SUSHI) {
            // V2 swap
            address[] memory path = new address[](2);
            path[0] = tokenIn;
            path[1] = tokenOut;
            
            uint[] memory amounts = IUniswapV2Router(router).swapExactTokensForTokens(
                amountIn,
                0, // Accept any amount
                path,
                address(this),
                block.timestamp + 300
            );
            
            return amounts[1];
            
        } else if (routerType == RouterType.V3_UNISWAP) {
            // V3 swap
            ISwapRouter.ExactInputSingleParams memory params = ISwapRouter.ExactInputSingleParams({
                tokenIn: tokenIn,
                tokenOut: tokenOut,
                fee: v3Fee,
                recipient: address(this),
                deadline: block.timestamp + 300,
                amountIn: amountIn,
                amountOutMinimum: 0,
                sqrtPriceLimitX96: 0
            });
            
            return ISwapRouter(router).exactInputSingle(params);
            
        } else if (routerType == RouterType.STABLE_DYSTOPIA) {
            // Dystopia stable swap
            IDystopiaRouter.route[] memory routes = new IDystopiaRouter.route[](1);
            routes[0] = IDystopiaRouter.route({
                from: tokenIn,
                to: tokenOut,
                stable: true // For stable pools
            });
            
            uint[] memory amounts = IDystopiaRouter(router).swapExactTokensForTokens(
                amountIn,
                0,
                routes,
                address(this),
                block.timestamp + 300
            );
            
            return amounts[1];
        }
        
        revert("Unknown router type");
    }
    
    function getRouter(RouterType routerType) internal pure returns (address) {
        if (routerType == RouterType.V2_QUICKSWAP) return QUICKSWAP_ROUTER;
        if (routerType == RouterType.V2_SUSHI) return SUSHISWAP_ROUTER;
        if (routerType == RouterType.V3_UNISWAP) return UNISWAP_V3_ROUTER;
        if (routerType == RouterType.STABLE_DYSTOPIA) return DYSTOPIA_ROUTER;
        revert("Invalid router type");
    }
    
    // Execute with separate V3 fees for buy and sell pools
    function executeArbitrageWithFees(
        address tokenA,
        address tokenB,
        uint256 amountIn,
        address buyPool,
        RouterType buyRouterType,
        uint24 buyV3Fee,
        address sellPool,
        RouterType sellRouterType,
        uint24 sellV3Fee
    ) external onlyOwner {
        // Approve routers
        address buyRouter = getRouter(buyRouterType);
        address sellRouter = getRouter(sellRouterType);
        
        IERC20(tokenA).approve(buyRouter, amountIn);
        
        // Execute buy swap with specific fee
        uint256 tokenBReceived = executeSwap(
            tokenA,
            tokenB,
            amountIn,
            buyRouter,
            buyRouterType,
            buyV3Fee
        );
        
        // Approve for sell swap
        IERC20(tokenB).approve(sellRouter, tokenBReceived);
        
        // Execute sell swap with specific fee
        uint256 tokenAReceived = executeSwap(
            tokenB,
            tokenA,
            tokenBReceived,
            sellRouter,
            sellRouterType,
            sellV3Fee
        );
        
        // Profit is tokenAReceived - amountIn
        require(tokenAReceived > amountIn, "No profit");
        
        // Send profit to owner
        IERC20(tokenA).transfer(owner, tokenAReceived);
    }
    
    // Simplified execution for common case
    function executeSimpleArbitrage(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        bool useQuickswapFirst,
        bool useV3Second
    ) external onlyOwner {
        // Common pattern: V2 -> V3 or V2 -> V2
        RouterType firstRouter = useQuickswapFirst ? RouterType.V2_QUICKSWAP : RouterType.V2_SUSHI;
        RouterType secondRouter = useV3Second ? RouterType.V3_UNISWAP : RouterType.V2_QUICKSWAP;
        
        // For V3, default to 0.3% fee
        uint24 v3Fee = 3000;
        
        executeArbitrage(
            tokenIn,
            tokenOut,
            amountIn,
            address(0), // Pool addresses not needed for router-based execution
            firstRouter,
            address(0),
            secondRouter,
            v3Fee
        );
    }
    
    // Withdraw any stuck tokens
    function withdraw(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        if (balance > 0) {
            IERC20(token).transfer(owner, balance);
        }
    }
    
    receive() external payable {}
}