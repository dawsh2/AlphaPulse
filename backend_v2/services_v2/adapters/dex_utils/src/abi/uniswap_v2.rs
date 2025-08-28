//! Uniswap V2 and compatible protocol ABIs
//!
//! This module contains the canonical ABI definitions for Uniswap V2
//! and compatible protocols (Sushiswap, Quickswap V2, etc).

use ethabi::{Event, EventParam, ParamType};

/// Uniswap V2/QuickSwap Swap event ABI definition  
/// event Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to)
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
                name: "amount0In".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "amount1In".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "amount0Out".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "amount1Out".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
            EventParam {
                name: "to".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
        ],
        anonymous: false,
    }
}

/// Uniswap V2 Mint event ABI definition
/// event Mint(address indexed sender, uint256 amount0, uint256 amount1)
pub fn mint_event() -> Event {
    Event {
        name: "Mint".to_string(),
        inputs: vec![
            EventParam {
                name: "sender".to_string(),
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
        ],
        anonymous: false,
    }
}

/// Uniswap V2 Burn event ABI definition
/// event Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to)
pub fn burn_event() -> Event {
    Event {
        name: "Burn".to_string(),
        inputs: vec![
            EventParam {
                name: "sender".to_string(),
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
                name: "to".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
        ],
        anonymous: false,
    }
}

/// Uniswap V2 Sync event ABI definition
/// event Sync(uint112 reserve0, uint112 reserve1)
pub fn sync_event() -> Event {
    Event {
        name: "Sync".to_string(),
        inputs: vec![
            EventParam {
                name: "reserve0".to_string(),
                kind: ParamType::Uint(112),
                indexed: false,
            },
            EventParam {
                name: "reserve1".to_string(),
                kind: ParamType::Uint(112),
                indexed: false,
            },
        ],
        anonymous: false,
    }
}

/// Uniswap V2 Pair Creation event ABI definition (from Factory)
/// event PairCreated(address indexed token0, address indexed token1, address pair, uint256)
pub fn pair_created_event() -> Event {
    Event {
        name: "PairCreated".to_string(),
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
                name: "pair".to_string(),
                kind: ParamType::Address,
                indexed: false,
            },
            EventParam {
                name: "".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
        ],
        anonymous: false,
    }
}
