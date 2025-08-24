//! Uniswap V3 and compatible protocol ABIs
//!
//! This module contains the canonical ABI definitions for Uniswap V3
//! and compatible protocols (Quickswap V3, etc).

use ethabi::{Event, EventParam, ParamType};

/// Uniswap V3 Swap event ABI definition
/// event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
pub fn swap_event() -> Event {
    Event {
        name: "Swap".to_string(),
        inputs: vec![
            EventParam {
                name: "sender".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "recipient".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "amount0".to_string(),
                kind: ParamType::Int(256),
                indexed: false,
            },
            EventParam {
                name: "amount1".to_string(),
                kind: ParamType::Int(256),
                indexed: false,
            },
            EventParam {
                name: "sqrtPriceX96".to_string(),
                kind: ParamType::Uint(160),
                indexed: false,
            },
            EventParam {
                name: "liquidity".to_string(),
                kind: ParamType::Uint(128),
                indexed: false,
            },
            EventParam {
                name: "tick".to_string(),
                kind: ParamType::Int(24),
                indexed: false,
            },
        ],
        anonymous: false,
    }
}

/// Uniswap V3 Mint event ABI definition
/// event Mint(address sender, address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
pub fn mint_event() -> Event {
    Event {
        name: "Mint".to_string(),
        inputs: vec![
            EventParam {
                name: "sender".to_string(),
                kind: ParamType::Address,
                indexed: false,
            },
            EventParam {
                name: "owner".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "tickLower".to_string(),
                kind: ParamType::Int(24),
                indexed: true,
            },
            EventParam {
                name: "tickUpper".to_string(),
                kind: ParamType::Int(24),
                indexed: true,
            },
            EventParam {
                name: "amount".to_string(),
                kind: ParamType::Uint(128),
                indexed: false,
            },
            EventParam {
                name: "amount0".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "amount1".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
        ],
        anonymous: false,
    }
}

/// Uniswap V3 Burn event ABI definition
/// event Burn(address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
pub fn burn_event() -> Event {
    Event {
        name: "Burn".to_string(),
        inputs: vec![
            EventParam {
                name: "owner".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "tickLower".to_string(),
                kind: ParamType::Int(24),
                indexed: true,
            },
            EventParam {
                name: "tickUpper".to_string(),
                kind: ParamType::Int(24),
                indexed: true,
            },
            EventParam {
                name: "amount".to_string(),
                kind: ParamType::Uint(128),
                indexed: false,
            },
            EventParam {
                name: "amount0".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "amount1".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
        ],
        anonymous: false,
    }
}

/// Uniswap V3 Initialize event ABI definition
/// event Initialize(uint160 sqrtPriceX96, int24 tick)
pub fn initialize_event() -> Event {
    Event {
        name: "Initialize".to_string(),
        inputs: vec![
            EventParam {
                name: "sqrtPriceX96".to_string(),
                kind: ParamType::Uint(160),
                indexed: false,
            },
            EventParam {
                name: "tick".to_string(),
                kind: ParamType::Int(24),
                indexed: false,
            },
        ],
        anonymous: false,
    }
}

/// Uniswap V3 Flash event ABI definition
/// event Flash(address indexed sender, address indexed recipient, uint256 amount0, uint256 amount1, uint256 paid0, uint256 paid1)
pub fn flash_event() -> Event {
    Event {
        name: "Flash".to_string(),
        inputs: vec![
            EventParam {
                name: "sender".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "recipient".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "amount0".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "amount1".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "paid0".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "paid1".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
        ],
        anonymous: false,
    }
}

/// Uniswap V3 Pool Creation event ABI definition (from Factory)
/// event PoolCreated(address indexed token0, address indexed token1, uint24 indexed fee, int24 tickSpacing, address pool)
pub fn pool_created_event() -> Event {
    Event {
        name: "PoolCreated".to_string(),
        inputs: vec![
            EventParam {
                name: "token0".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "token1".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "fee".to_string(),
                kind: ParamType::Uint(24),
                indexed: true,
            },
            EventParam {
                name: "tickSpacing".to_string(),
                kind: ParamType::Int(24),
                indexed: false,
            },
            EventParam {
                name: "pool".to_string(),
                kind: ParamType::Address,
                indexed: false,
            },
        ],
        anonymous: false,
    }
}
