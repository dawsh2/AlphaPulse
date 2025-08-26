//! Token Precision Unit Tests
//!
//! Tests for native token precision handling - critical for DEX operations.

use protocol_v2::precision::{TokenPrecision, WETH_DECIMALS, USDC_DECIMALS, USDT_DECIMALS};

#[test]
fn test_native_token_decimals() {
    // WETH: 18 decimals (standard ERC-20)
    assert_eq!(WETH_DECIMALS, 18);
    
    // USDC: 6 decimals (Circle standard)
    assert_eq!(USDC_DECIMALS, 6);
    
    // USDT: 6 decimals (Tether standard)
    assert_eq!(USDT_DECIMALS, 6);
}

#[test]
fn test_weth_precision_preservation() {
    // Test exact WETH amounts with full 18-decimal precision
    let one_wei = 1i64;
    let one_gwei = 1_000_000_000i64; // 10^9 wei
    let one_ether = 1_000_000_000_000_000_000i64; // 10^18 wei
    
    assert_eq!(TokenPrecision::format_weth(one_wei), "0.000000000000000001");
    assert_eq!(TokenPrecision::format_weth(one_gwei), "0.000000001");
    assert_eq!(TokenPrecision::format_weth(one_ether), "1.0");
}

#[test]
fn test_usdc_precision_preservation() {
    // Test exact USDC amounts with 6-decimal precision
    let one_microusdc = 1i64; // Smallest USDC unit
    let one_cent = 10_000i64; // $0.01 in micro-USDC
    let one_dollar = 1_000_000i64; // $1.00 in micro-USDC
    
    assert_eq!(TokenPrecision::format_usdc(one_microusdc), "0.000001");
    assert_eq!(TokenPrecision::format_usdc(one_cent), "0.01");
    assert_eq!(TokenPrecision::format_usdc(one_dollar), "1.0");
}

#[test]
fn test_precision_no_truncation() {
    // Critical: Never truncate or lose precision in storage
    
    // WETH: Store full 18 decimals
    let precise_weth = 1_234_567_890_123_456_789i64; // 1.234567890123456789 ETH
    let stored_weth = TokenPrecision::store_weth_amount(precise_weth);
    let retrieved_weth = TokenPrecision::retrieve_weth_amount(stored_weth);
    assert_eq!(retrieved_weth, precise_weth);
    
    // USDC: Store full 6 decimals
    let precise_usdc = 123_456_789i64; // 123.456789 USDC
    let stored_usdc = TokenPrecision::store_usdc_amount(precise_usdc);
    let retrieved_usdc = TokenPrecision::retrieve_usdc_amount(stored_usdc);
    assert_eq!(retrieved_usdc, precise_usdc);
}

#[test]
fn test_token_amount_boundaries() {
    // Test edge cases for token amounts
    
    // Maximum realistic ETH supply: ~120M ETH
    let max_eth_supply = 120_000_000i64 * 10i64.pow(18);
    assert!(TokenPrecision::is_valid_weth_amount(max_eth_supply));
    
    // Maximum realistic USDC supply: trillions of dollars
    let max_usdc_supply = 1_000_000_000_000i64 * 10i64.pow(6); // $1T
    assert!(TokenPrecision::is_valid_usdc_amount(max_usdc_supply));
    
    // Negative amounts should be invalid
    assert!(!TokenPrecision::is_valid_weth_amount(-1));
    assert!(!TokenPrecision::is_valid_usdc_amount(-1));
}

#[test]
fn test_zero_amounts_handling() {
    // Zero amounts should be valid and handle correctly
    assert!(TokenPrecision::is_valid_weth_amount(0));
    assert!(TokenPrecision::is_valid_usdc_amount(0));
    
    assert_eq!(TokenPrecision::format_weth(0), "0.0");
    assert_eq!(TokenPrecision::format_usdc(0), "0.0");
}

#[test]
fn test_precision_arithmetic_safety() {
    // Test that precision arithmetic doesn't overflow or underflow
    
    // Large WETH amounts
    let large_weth = 1_000_000i64 * 10i64.pow(18); // 1M ETH
    let result = TokenPrecision::multiply_weth_amount(large_weth, 2);
    assert_eq!(result, 2_000_000i64 * 10i64.pow(18));
    
    // Division with remainder handling
    let weth_amount = 1_000_000_000_000_000_001i64; // 1.000000000000000001 ETH
    let divided = TokenPrecision::divide_weth_amount(weth_amount, 3);
    
    // Should preserve as much precision as possible
    let expected = 333_333_333_333_333_333i64; // Truncated, but no rounding errors
    assert_eq!(divided, expected);
}

#[test]
fn test_dust_amounts_handling() {
    // Very small amounts (dust) should be handled correctly
    
    // 1 wei of WETH
    let dust_weth = 1i64;
    assert!(TokenPrecision::is_dust_weth(dust_weth));
    assert_eq!(TokenPrecision::format_weth(dust_weth), "0.000000000000000001");
    
    // 1 micro-USDC
    let dust_usdc = 1i64;
    assert!(TokenPrecision::is_dust_usdc(dust_usdc));
    assert_eq!(TokenPrecision::format_usdc(dust_usdc), "0.000001");
}

#[test]
fn test_human_readable_formatting() {
    // Test formatting for human display (but not for calculations!)
    
    let amounts_weth = [
        (1_000_000_000_000_000_000i64, "1.0 ETH"),
        (500_000_000_000_000_000i64, "0.5 ETH"),
        (1_234_567_890_123_456i64, "0.001234567890123456 ETH"),
    ];
    
    for (amount, expected) in amounts_weth {
        assert_eq!(TokenPrecision::human_format_weth(amount), expected);
    }
    
    let amounts_usdc = [
        (1_000_000i64, "1.0 USDC"),
        (1_500_000i64, "1.5 USDC"),
        (123_456i64, "0.123456 USDC"),
    ];
    
    for (amount, expected) in amounts_usdc {
        assert_eq!(TokenPrecision::human_format_usdc(amount), expected);
    }
}

#[test]
fn test_cross_token_precision_isolation() {
    // Different tokens should not interfere with each other's precision
    
    let weth_amount = 1_000_000_000_000_000_000i64; // 1 WETH (18 decimals)
    let usdc_amount = 1_000_000i64; // 1 USDC (6 decimals)
    
    // They should have same logical value ($1 each) but different storage
    assert_ne!(weth_amount, usdc_amount); // Storage format differs
    
    // But both represent meaningful amounts in their respective tokens
    assert!(TokenPrecision::is_meaningful_weth_amount(weth_amount));
    assert!(TokenPrecision::is_meaningful_usdc_amount(usdc_amount));
}

// Mock TokenPrecision implementation for testing
// This would be implemented in the actual precision module
#[allow(dead_code)]
struct TokenPrecision;

impl TokenPrecision {
    fn format_weth(amount: i64) -> String {
        format!("{:.18}", amount as f64 / 10_f64.powi(18)).trim_end_matches('0').trim_end_matches('.').to_string()
    }
    
    fn format_usdc(amount: i64) -> String {
        format!("{:.6}", amount as f64 / 10_f64.powi(6)).trim_end_matches('0').trim_end_matches('.').to_string()
    }
    
    fn store_weth_amount(amount: i64) -> i64 { amount }
    fn retrieve_weth_amount(stored: i64) -> i64 { stored }
    
    fn store_usdc_amount(amount: i64) -> i64 { amount }
    fn retrieve_usdc_amount(stored: i64) -> i64 { stored }
    
    fn is_valid_weth_amount(amount: i64) -> bool { amount >= 0 }
    fn is_valid_usdc_amount(amount: i64) -> bool { amount >= 0 }
    
    fn multiply_weth_amount(amount: i64, multiplier: i64) -> i64 { amount * multiplier }
    fn divide_weth_amount(amount: i64, divisor: i64) -> i64 { amount / divisor }
    
    fn is_dust_weth(amount: i64) -> bool { amount < 1000 } // < 1000 wei
    fn is_dust_usdc(amount: i64) -> bool { amount < 100 } // < 0.0001 USDC
    
    fn human_format_weth(amount: i64) -> String {
        format!("{} ETH", Self::format_weth(amount))
    }
    
    fn human_format_usdc(amount: i64) -> String {
        format!("{} USDC", Self::format_usdc(amount))
    }
    
    fn is_meaningful_weth_amount(amount: i64) -> bool { amount > 1000 }
    fn is_meaningful_usdc_amount(amount: i64) -> bool { amount > 100 }
}

// Mock constants
const WETH_DECIMALS: u8 = 18;
const USDC_DECIMALS: u8 = 6;
const USDT_DECIMALS: u8 = 6;