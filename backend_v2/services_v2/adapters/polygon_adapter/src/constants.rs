//! Polygon blockchain constants and event signatures
//!
//! These constants are defined by smart contract ABIs and blockchain protocols.
//! They are immutable once deployed and are not configuration values.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Well-known event signatures (topic[0]) for DEX protocols
pub mod event_signatures {
    /// Uniswap V2 Swap event signature
    /// event Swap(address indexed sender, uint amount0In, uint amount1In, uint amount0Out, uint amount1Out, address indexed to)
    pub const UNISWAP_V2_SWAP: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
    
    /// Uniswap V3 Swap event signature  
    /// event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
    pub const UNISWAP_V3_SWAP: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
    
    /// Uniswap V3 Mint event signature
    pub const UNISWAP_V3_MINT: &str = "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde";
    
    /// Uniswap V3 Burn event signature
    pub const UNISWAP_V3_BURN: &str = "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c";
    
    /// Sync event for liquidity pool reserves update
    pub const SYNC: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";
}

/// Well-known token addresses on Polygon network
pub mod token_addresses {
    /// USDC on Polygon
    pub const USDC: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
    
    /// WMATIC (Wrapped MATIC) on Polygon
    pub const WMATIC: &str = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270";
    
    /// WETH (Wrapped ETH) on Polygon
    pub const WETH: &str = "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619";
    
    /// DAI on Polygon
    pub const DAI: &str = "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063";
    
    /// WBTC (Wrapped Bitcoin) on Polygon
    pub const WBTC: &str = "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6";
}

/// DEX router addresses on Polygon
pub mod dex_routers {
    /// QuickSwap V2 Router
    pub const QUICKSWAP_V2: &str = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff";
    
    /// SushiSwap Router
    pub const SUSHISWAP: &str = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506";
    
    /// Uniswap V3 SwapRouter
    pub const UNISWAP_V3: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
}

/// Token metadata cache for common tokens
pub static TOKEN_SYMBOLS: Lazy<HashMap<u64, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Use first 8 bytes of address as key for efficient lookup
    m.insert(0x2791bca1f2de4661u64, "USDC");
    m.insert(0x0d500b1d8e8ef31eu64, "WMATIC");
    m.insert(0x7ceb23fd6bc0add5u64, "WETH");
    m.insert(0x8f3cf7ad23cd3cadu64, "DAI");
    m.insert(0x1bfd67037b42cf73u64, "WBTC");
    m
});

/// Get all monitored event signatures for subscription
pub fn get_monitored_event_signatures() -> Vec<&'static str> {
    vec![
        event_signatures::UNISWAP_V2_SWAP,
        event_signatures::UNISWAP_V3_SWAP,
        event_signatures::UNISWAP_V3_MINT,
        event_signatures::UNISWAP_V3_BURN,
        event_signatures::SYNC,
    ]
}