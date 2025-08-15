use anyhow::{Context, Result};
use ethers::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

// Polygon DEX Router addresses
pub const QUICKSWAP_ROUTER: &str = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff";
pub const SUSHISWAP_ROUTER: &str = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506";
pub const UNISWAP_V3_ROUTER: &str = "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45";

// Common Polygon tokens
pub const WMATIC: &str = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270";
pub const USDC: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";      // Native USDC
pub const USDC_E: &str = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359";     // Bridged USDC (USDC.e)
pub const USDT: &str = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F";
pub const WETH: &str = "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619";
pub const DAI: &str = "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063";

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Debug, Clone)]
pub struct DexRouter {
    pub address: Address,
    pub name: String,
    pub fee_bps: u16,  // Fee in basis points (e.g., 30 = 0.3%)
}

pub struct DexManager {
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    routers: HashMap<Address, DexRouter>,
    tokens: HashMap<Address, TokenInfo>,
}

impl DexManager {
    pub fn new(client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>) -> Self {
        let mut routers = HashMap::new();
        let mut tokens = HashMap::new();

        // Initialize routers
        routers.insert(
            QUICKSWAP_ROUTER.parse().unwrap(),
            DexRouter {
                address: QUICKSWAP_ROUTER.parse().unwrap(),
                name: "QuickSwap".to_string(),
                fee_bps: 30,
            },
        );
        
        routers.insert(
            SUSHISWAP_ROUTER.parse().unwrap(),
            DexRouter {
                address: SUSHISWAP_ROUTER.parse().unwrap(),
                name: "SushiSwap".to_string(),
                fee_bps: 30,
            },
        );

        routers.insert(
            UNISWAP_V3_ROUTER.parse().unwrap(),
            DexRouter {
                address: UNISWAP_V3_ROUTER.parse().unwrap(),
                name: "Uniswap V3".to_string(),
                fee_bps: 30,
            },
        );

        // Initialize tokens
        tokens.insert(
            WMATIC.parse().unwrap(),
            TokenInfo {
                address: WMATIC.parse().unwrap(),
                symbol: "WMATIC".to_string(),
                decimals: 18,
            },
        );

        tokens.insert(
            USDC.parse().unwrap(),
            TokenInfo {
                address: USDC.parse().unwrap(),
                symbol: "USDC".to_string(),
                decimals: 6,
            },
        );

        tokens.insert(
            USDT.parse().unwrap(),
            TokenInfo {
                address: USDT.parse().unwrap(),
                symbol: "USDT".to_string(),
                decimals: 6,
            },
        );

        tokens.insert(
            WETH.parse().unwrap(),
            TokenInfo {
                address: WETH.parse().unwrap(),
                symbol: "WETH".to_string(),
                decimals: 18,
            },
        );

        tokens.insert(
            DAI.parse().unwrap(),
            TokenInfo {
                address: DAI.parse().unwrap(),
                symbol: "DAI".to_string(),
                decimals: 18,
            },
        );

        Self {
            client,
            routers,
            tokens,
        }
    }

    pub fn get_router(&self, address: &Address) -> Option<&DexRouter> {
        self.routers.get(address)
    }

    pub fn get_token(&self, address: &Address) -> Option<&TokenInfo> {
        self.tokens.get(address)
    }

    pub fn get_router_name(&self, address: &Address) -> String {
        self.routers
            .get(address)
            .map(|r| r.name.clone())
            .unwrap_or_else(|| format!("{:?}", address))
    }

    pub fn get_token_symbol(&self, address: &Address) -> String {
        self.tokens
            .get(address)
            .map(|t| t.symbol.clone())
            .unwrap_or_else(|| format!("{:?}", address))
    }
}

// Uniswap V2 Router ABI (simplified)
abigen!(
    IUniswapV2Router,
    r#"[
        function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)
        function WETH() external pure returns (address)
    ]"#
);

// ERC20 Token ABI (simplified)
abigen!(
    IERC20,
    r#"[
        function balanceOf(address owner) external view returns (uint256)
        function approve(address spender, uint256 amount) external returns (bool)
        function transfer(address to, uint256 amount) external returns (bool)
        function decimals() external view returns (uint8)
        function symbol() external view returns (string)
    ]"#
);