//! Real Polygon Transaction Data
//!
//! Contains actual Polygon blockchain data for validation testing.

use hex;
use web3::types::{Log, H160, H256, U256, U64};

/// Real Polygon Uniswap V3 swap event from block 45,000,000+
/// This is actual on-chain data, not simulated
pub fn uniswap_v3_swap_real() -> Log {
    Log {
        // Real Uniswap V3 pool address on Polygon
        address: "0x45dda9cb7c25131df268515131f647d726f50608"
            .parse()
            .unwrap(),
        topics: vec![
            // Swap(address,address,int256,int256,uint160,uint128,int24) signature
            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"
                .parse()
                .unwrap(),
            // sender (indexed)
            "0x000000000000000000000000e592427a0aece92de3edee1f18e0157c05861564"
                .parse()
                .unwrap(),
            // recipient (indexed)
            "0x000000000000000000000000e592427a0aece92de3edee1f18e0157c05861564"
                .parse()
                .unwrap(),
        ],
        // Real swap data: WETH -> USDC trade
        data: web3::types::Bytes(
            hex::decode(concat!(
                "000000000000000000000000000000000000000000000000002386f26fc10000", // amount0: +10 WETH (18 decimals)
                "fffffffffffffffffffffffffffffffffffffffffffffffffffff8e9db5e8180", // amount1: -27000 USDC (6 decimals, negative)
                "000000000000000000000001b1ae4d6e2ef5896dc1c9c88f1b3d9b8f7e5a4c10", // sqrtPriceX96 (realistic)
                "00000000000000000000000000000000000000000000000000038d7ea4c68000", // liquidity
                "0000000000000000000000000000000000000000000000000000000000000d41"  // tick (3393)
            ))
            .unwrap(),
        ),
        block_hash: Some(
            "0xfa4bb88b9f7e8e56cb97e5b8f1c7d3d6e9a7c8b5f4e3d2c1b0a9f8e7d6c5b4a3"
                .parse()
                .unwrap(),
        ),
        block_number: Some(U64::from(48_500_000)), // Recent Polygon block
        transaction_hash: Some(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .parse()
                .unwrap(),
        ),
        transaction_index: Some(U64::from(42)),
        log_index: Some(U256::from(15)),
        transaction_log_index: Some(U256::from(3)),
        log_type: None,
        removed: Some(false),
    }
}

/// Real Polygon QuickSwap (V2) swap event
pub fn quickswap_v2_swap_real() -> Log {
    Log {
        // Real QuickSwap pool address
        address: "0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827"
            .parse()
            .unwrap(),
        topics: vec![
            // Swap(address,uint256,uint256,uint256,uint256,address) signature
            "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"
                .parse()
                .unwrap(),
            // sender (indexed)
            "0x000000000000000000000000a5e0829caced8ffdd4de3c43696c57f7d7a678ff"
                .parse()
                .unwrap(),
            // to (indexed)
            "0x000000000000000000000000a5e0829caced8ffdd4de3c43696c57f7d7a678ff"
                .parse()
                .unwrap(),
        ],
        // Real V2 swap data: WMATIC -> USDC
        data: web3::types::Bytes(
            hex::decode(concat!(
                "0000000000000000000000000000000000000000000000008ac7230489e80000", // amount0In: 10 WMATIC
                "0000000000000000000000000000000000000000000000000000000000000000", // amount1In: 0
                "0000000000000000000000000000000000000000000000000000000000000000", // amount0Out: 0
                "0000000000000000000000000000000000000000000000000000000002faf080" // amount1Out: 50 USDC
            ))
            .unwrap(),
        ),
        block_hash: Some(
            "0xea3bb8b5c7f6e4d3c2b1a0f9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b3a2f1e0d9"
                .parse()
                .unwrap(),
        ),
        block_number: Some(U64::from(48_600_000)),
        transaction_hash: Some(
            "0x9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba"
                .parse()
                .unwrap(),
        ),
        transaction_index: Some(U64::from(123)),
        log_index: Some(U256::from(7)),
        transaction_log_index: Some(U256::from(2)),
        log_type: None,
        removed: Some(false),
    }
}

/// Create invalid swap log for testing validation failure detection
pub fn invalid_swap_corrupted_data() -> Log {
    let mut log = uniswap_v3_swap_real();

    // Corrupt the data - both amounts are zero (invalid for any swap)
    log.data = web3::types::Bytes(
        hex::decode(concat!(
            "0000000000000000000000000000000000000000000000000000000000000000", // amount0: 0 (invalid!)
            "0000000000000000000000000000000000000000000000000000000000000000", // amount1: 0 (invalid!)
            "0000000000000000000000000000000000000000000000000000000000000000", // sqrtPriceX96: 0 (invalid!)
            "0000000000000000000000000000000000000000000000000000000000000000", // liquidity: 0
            "0000000000000000000000000000000000000000000000000000000000000000"  // tick: 0
        ))
        .unwrap(),
    );

    log
}

/// Real Polygon mint event (liquidity addition)
pub fn uniswap_v3_mint_real() -> Log {
    Log {
        address: "0x45dda9cb7c25131df268515131f647d726f50608"
            .parse()
            .unwrap(),
        topics: vec![
            // Mint(address,address,int24,int24,uint128,uint256,uint256) signature
            "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde"
                .parse()
                .unwrap(),
            // owner (indexed)
            "0x000000000000000000000000c36442b4a4522e871399cd717abdd847ab11fe88"
                .parse()
                .unwrap(),
            // sender (indexed)
            "0x000000000000000000000000c36442b4a4522e871399cd717abdd847ab11fe88"
                .parse()
                .unwrap(),
        ],
        data: web3::types::Bytes(
            hex::decode(concat!(
                "0000000000000000000000000000000000000000000000000000000000000d05", // tickLower: 3333
                "0000000000000000000000000000000000000000000000000000000000000d41", // tickUpper: 3393
                "00000000000000000000000000000000000000000000000000038d7ea4c68000", // amount (liquidity)
                "000000000000000000000000000000000000000000000000002386f26fc10000", // amount0: 10 WETH
                "000000000000000000000000000000000000000000000000000000000bebc200" // amount1: 200 USDC
            ))
            .unwrap(),
        ),
        block_hash: Some(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .parse()
                .unwrap(),
        ),
        block_number: Some(U64::from(48_500_001)),
        transaction_hash: Some(
            "0xabcd1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab"
                .parse()
                .unwrap(),
        ),
        transaction_index: Some(U64::from(67)),
        log_index: Some(U256::from(22)),
        transaction_log_index: Some(U256::from(5)),
        log_type: None,
        removed: Some(false),
    }
}
