//! Tests for ethabi-based event parsing
//!
//! Verifies that our ethabi event decoding correctly parses
//! real Polygon DEX events without manual byte slicing risks.

use ethabi::{Event, EventParam, ParamType, RawLog};
use lazy_static::lazy_static;
use web3::types::{Bytes, Log, H256, U64};

lazy_static! {
    /// Uniswap V3 Swap event ABI
    static ref UNISWAP_V3_SWAP_EVENT: Event = Event {
        name: "Swap".to_string(),
        inputs: vec![
            EventParam { name: "sender".to_string(), kind: ParamType::Address, indexed: true },
            EventParam { name: "recipient".to_string(), kind: ParamType::Address, indexed: true },
            EventParam { name: "amount0".to_string(), kind: ParamType::Int(256), indexed: false },
            EventParam { name: "amount1".to_string(), kind: ParamType::Int(256), indexed: false },
            EventParam { name: "sqrtPriceX96".to_string(), kind: ParamType::Uint(160), indexed: false },
            EventParam { name: "liquidity".to_string(), kind: ParamType::Uint(128), indexed: false },
            EventParam { name: "tick".to_string(), kind: ParamType::Int(24), indexed: false },
        ],
        anonymous: false,
    };

    /// V2 Swap event ABI
    static ref UNISWAP_V2_SWAP_EVENT: Event = Event {
        name: "Swap".to_string(),
        inputs: vec![
            EventParam { name: "sender".to_string(), kind: ParamType::Address, indexed: true },
            EventParam { name: "amount0In".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "amount1In".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "amount0Out".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "amount1Out".to_string(), kind: ParamType::Uint(256), indexed: false },
            EventParam { name: "to".to_string(), kind: ParamType::Address, indexed: true },
        ],
        anonymous: false,
    };
}

#[test]
fn test_v3_swap_event_parsing() {
    // Create a realistic V3 swap log
    // Event data: amount0=-1000000, amount1=2000000000, sqrtPriceX96=79228162514264337593543950336, liquidity=1000000000000, tick=100
    let log = Log {
        address: "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640"
            .parse()
            .unwrap(),
        topics: vec![
            // Event signature
            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"
                .parse::<H256>()
                .unwrap(),
            // Indexed sender address
            "0x000000000000000000000000def1c0ded9bec7f1a1670819833240f027b25eff"
                .parse()
                .unwrap(),
            // Indexed recipient address
            "0x00000000000000000000000068b3465833fb72a70ecdf485e0e4c7bd8665fc45"
                .parse()
                .unwrap(),
        ],
        data: Bytes(
            hex::decode(concat!(
                "fffffffffffffffffffffffffffffffffffffffffffffffffffff0bdc2300000", // amount0 = -1000000000 (int256)
                "0000000000000000000000000000000000000000000000000000000077359400", // amount1 = 2000000000 (int256)
                "00000000000000000000000000000000000000001000000000000000000000000", // sqrtPriceX96 (uint160)
                "00000000000000000000000000000000000000000000000000000000e8d4a51000", // liquidity (uint128)
                "0000000000000000000000000000000000000000000000000000000000000064", // tick = 100 (int24)
            ))
            .unwrap(),
        ),
        block_number: Some(U64::from(18000000)),
        block_hash: None,
        transaction_hash: None,
        transaction_index: None,
        log_index: None,
        transaction_log_index: None,
        log_type: None,
        removed: None,
    };

    // Parse with ethabi
    let raw_log = RawLog {
        topics: log.topics.clone(),
        data: log.data.0.clone(),
    };

    let parsed = UNISWAP_V3_SWAP_EVENT.parse_log(raw_log).unwrap();

    // Verify parsed values
    let amount0 = parsed
        .params
        .iter()
        .find(|p| p.name == "amount0")
        .and_then(|p| p.value.clone().into_int())
        .unwrap();

    let amount1 = parsed
        .params
        .iter()
        .find(|p| p.name == "amount1")
        .and_then(|p| p.value.clone().into_int())
        .unwrap();

    let tick = parsed
        .params
        .iter()
        .find(|p| p.name == "tick")
        .and_then(|p| p.value.clone().into_int())
        .map(|v| v.low_u32() as i32)
        .unwrap();

    // amount0 should be negative (token0 out) - check if high bit is set (two's complement)
    assert!(amount0.bit(255)); // Sign bit set = negative

    // amount1 should be positive (token1 in)
    assert!(!amount1.bit(255)); // Sign bit not set = positive
    assert_eq!(amount1.low_u128(), 2000000000u128);

    // tick should be 100
    assert_eq!(tick, 100);
}

#[test]
fn test_v2_swap_event_parsing() {
    // Create a realistic V2 swap log
    let log = Log {
        address: "0xa5e0829caced8ffdd4de3c43696c57f7d7a678ff"
            .parse()
            .unwrap(),
        topics: vec![
            // Event signature
            "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"
                .parse::<H256>()
                .unwrap(),
            // Indexed sender
            "0x000000000000000000000000a5e0829caced8ffdd4de3c43696c57f7d7a678ff"
                .parse()
                .unwrap(),
            // Indexed to
            "0x00000000000000000000000068b3465833fb72a70ecdf485e0e4c7bd8665fc45"
                .parse()
                .unwrap(),
        ],
        data: Bytes(
            hex::decode(concat!(
                "0000000000000000000000000000000000000000000000000de0b6b3a7640000", // amount0In = 1e18
                "0000000000000000000000000000000000000000000000000000000000000000", // amount1In = 0
                "0000000000000000000000000000000000000000000000000000000000000000", // amount0Out = 0
                "000000000000000000000000000000000000000000000000000000003b9aca00", // amount1Out = 1e9
            ))
            .unwrap(),
        ),
        block_number: Some(U64::from(50000000)),
        block_hash: None,
        transaction_hash: None,
        transaction_index: None,
        log_index: None,
        transaction_log_index: None,
        log_type: None,
        removed: None,
    };

    // Parse with ethabi
    let raw_log = RawLog {
        topics: log.topics.clone(),
        data: log.data.0.clone(),
    };

    let parsed = UNISWAP_V2_SWAP_EVENT.parse_log(raw_log).unwrap();

    // Verify parsed values
    let amount0_in = parsed
        .params
        .iter()
        .find(|p| p.name == "amount0In")
        .and_then(|p| p.value.clone().into_uint())
        .unwrap();

    let amount1_out = parsed
        .params
        .iter()
        .find(|p| p.name == "amount1Out")
        .and_then(|p| p.value.clone().into_uint())
        .unwrap();

    assert_eq!(amount0_in.low_u128(), 1000000000000000000u128); // 1e18
    assert_eq!(amount1_out.low_u128(), 1000000000u128); // 1e9
}

#[test]
fn test_malformed_event_handling() {
    // Create a log with insufficient data
    let log = Log {
        address: "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640"
            .parse()
            .unwrap(),
        topics: vec![
            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"
                .parse::<H256>()
                .unwrap(),
        ],
        data: Bytes(vec![0u8; 10]), // Too short for V3 swap
        block_number: Some(U64::from(18000000)),
        block_hash: None,
        transaction_hash: None,
        transaction_index: None,
        log_index: None,
        transaction_log_index: None,
        log_type: None,
        removed: None,
    };

    let raw_log = RawLog {
        topics: log.topics.clone(),
        data: log.data.0.clone(),
    };

    // Should fail to parse
    assert!(UNISWAP_V3_SWAP_EVENT.parse_log(raw_log).is_err());
}

#[test]
fn test_precision_preservation() {
    // Test that we don't lose precision with large uint256 values
    let large_amount = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    let data = hex::decode(format!(
        "{}", // amount0 = max uint256
        large_amount
    ))
    .unwrap();

    // This verifies ethabi can handle full uint256 range
    let token = ethabi::decode(&[ParamType::Uint(256)], &data).unwrap();
    let value = token[0].clone().into_uint().unwrap();

    // Should preserve all 256 bits
    assert_eq!(format!("{:064x}", value), large_amount);
}
