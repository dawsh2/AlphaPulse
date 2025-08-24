#!/usr/bin/env rust-script
//! Test ethabi to get correct Uniswap V2/V3 event signatures
//!
//! This test uses ethabi to generate proper event signatures for:
//! - Uniswap V2 Swap, Sync, Mint, Burn
//! - Uniswap V3 Swap, Mint, Burn

use ethabi::{Event, EventParam, ParamType};
use hex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Using ethabi to generate correct Uniswap event signatures\n");

    // Uniswap V2 Events
    println!("=== Uniswap V2 Events ===");

    // V2 Swap: Swap(indexed address sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, indexed address to)
    let v2_swap = Event {
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
    };

    // V2 Sync: Sync(uint112 reserve0, uint112 reserve1)
    let v2_sync = Event {
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
    };

    // V2 Mint: Mint(indexed address sender, uint256 amount0, uint256 amount1)
    let v2_mint = Event {
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
    };

    // V2 Burn: Burn(indexed address sender, uint256 amount0, uint256 amount1, indexed address to)
    let v2_burn = Event {
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
    };

    println!("V2 Swap signature: 0x{}", hex::encode(v2_swap.signature()));
    println!("V2 Sync signature: 0x{}", hex::encode(v2_sync.signature()));
    println!("V2 Mint signature: 0x{}", hex::encode(v2_mint.signature()));
    println!("V2 Burn signature: 0x{}", hex::encode(v2_burn.signature()));

    println!("\n=== Uniswap V3 Events ===");

    // V3 Swap: Swap(indexed address sender, indexed address recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
    let v3_swap = Event {
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
    };

    // V3 Mint: Mint(address sender, indexed address owner, indexed int24 tickLower, indexed int24 tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
    let v3_mint = Event {
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
    };

    // V3 Burn: Burn(indexed address owner, indexed int24 tickLower, indexed int24 tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
    let v3_burn = Event {
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
    };

    println!("V3 Swap signature: 0x{}", hex::encode(v3_swap.signature()));
    println!("V3 Mint signature: 0x{}", hex::encode(v3_mint.signature()));
    println!("V3 Burn signature: 0x{}", hex::encode(v3_burn.signature()));

    println!("\n=== ERC20 Transfer Event ===");

    // Transfer: Transfer(indexed address from, indexed address to, uint256 value)
    let transfer = Event {
        name: "Transfer".to_string(),
        inputs: vec![
            EventParam {
                name: "from".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "to".to_string(),
                kind: ParamType::Address,
                indexed: true,
            },
            EventParam {
                name: "value".to_string(),
                kind: ParamType::Uint(256),
                indexed: false,
            },
        ],
        anonymous: false,
    };

    println!(
        "ERC20 Transfer signature: 0x{}",
        hex::encode(transfer.signature())
    );

    println!("\n=== Comparison with our current config ===");
    println!("Current V2 swap: 0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67");
    println!("Ethabi  V2 swap: 0x{}", hex::encode(v2_swap.signature()));

    println!("Current V3 swap: 0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822");
    println!("Ethabi  V3 swap: 0x{}", hex::encode(v3_swap.signature()));

    println!("Current Sync:    0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1");
    println!("Ethabi  Sync:    0x{}", hex::encode(v2_sync.signature()));

    println!(
        "Current Transfer: 0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
    );
    println!("Ethabi  Transfer: 0x{}", hex::encode(transfer.signature()));

    Ok(())
}
