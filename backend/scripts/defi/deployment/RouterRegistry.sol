// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IFactory {
    function factory() external view returns (address);
}

interface IStablePool {
    function stable() external view returns (bool);
}

contract RouterRegistry {
    // Main routers on Polygon
    address constant QUICKSWAP_ROUTER = 0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff;
    address constant SUSHISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    address constant UNISWAP_V3_ROUTER = 0xE592427A0AEce92De3Edee1F18E0157C05861564;
    address constant DYSTOPIA_ROUTER = 0xbE75Dd16D029c6B32B7aD57A0FD9C1c20Dd2862e;
    address constant CURVE_ROUTER = 0x74dC1C4ec10abE9F5C8A3EabF1A90b97cDc3Ead8; // TriCrypto
    address constant BALANCER_VAULT = 0xBA12222222228d8Ba445958a75a0704d566BF2C8;
    
    // Factory addresses to identify DEX
    address constant QUICKSWAP_FACTORY = 0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32;
    address constant SUSHISWAP_FACTORY = 0xc35DADB65012eC5796536bD9864eD8773aBc74C4;
    address constant UNISWAP_V3_FACTORY = 0x1F98431c8aD98523631AE4a59f267346ea31F984;
    address constant DYSTOPIA_FACTORY = 0x1d21Db6cde1b18c7E47B0F7F42f4b3F68b9beeC9;
    
    struct RouterInfo {
        address router;
        string dexName;
        uint8 routerType; // 0=V2, 1=V3, 2=Stable, 3=Curve, 4=Balancer
    }
    
    mapping(address => RouterInfo) public poolRouters;
    address public owner;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function getRouterForPool(address pool) external view returns (
        address router,
        string memory dexName,
        uint8 routerType
    ) {
        // Check if already cached
        if (poolRouters[pool].router != address(0)) {
            RouterInfo memory info = poolRouters[pool];
            return (info.router, info.dexName, info.routerType);
        }
        
        // Auto-detect router
        (router, dexName, routerType) = detectRouter(pool);
        return (router, dexName, routerType);
    }
    
    function detectRouter(address pool) public view returns (
        address router,
        string memory dexName,
        uint8 routerType
    ) {
        // Try to get factory from pool
        try IFactory(pool).factory() returns (address factory) {
            // Check V2-style factories
            if (factory == QUICKSWAP_FACTORY) {
                return (QUICKSWAP_ROUTER, "QuickSwap", 0);
            } else if (factory == SUSHISWAP_FACTORY) {
                return (SUSHISWAP_ROUTER, "SushiSwap", 0);
            } else if (factory == DYSTOPIA_FACTORY) {
                // Check if it's a stable pool
                try IStablePool(pool).stable() returns (bool isStable) {
                    if (isStable) {
                        return (DYSTOPIA_ROUTER, "Dystopia-Stable", 2);
                    }
                } catch {}
                return (DYSTOPIA_ROUTER, "Dystopia", 0);
            }
        } catch {}
        
        // Check V3 by trying specific V3 functions
        try IUniswapV3Pool(pool).fee() returns (uint24) {
            return (UNISWAP_V3_ROUTER, "UniswapV3", 1);
        } catch {}
        
        // Default to QuickSwap for unknown V2 pools
        return (QUICKSWAP_ROUTER, "Unknown-V2", 0);
    }
    
    function cacheRouter(address pool, address router, string memory dexName, uint8 routerType) external onlyOwner {
        poolRouters[pool] = RouterInfo(router, dexName, routerType);
    }
    
    function cacheMultiple(
        address[] calldata pools,
        address[] calldata routers,
        string[] calldata dexNames,
        uint8[] calldata routerTypes
    ) external onlyOwner {
        require(pools.length == routers.length, "Length mismatch");
        for (uint i = 0; i < pools.length; i++) {
            poolRouters[pools[i]] = RouterInfo(routers[i], dexNames[i], routerTypes[i]);
        }
    }
}

interface IUniswapV3Pool {
    function fee() external view returns (uint24);
}