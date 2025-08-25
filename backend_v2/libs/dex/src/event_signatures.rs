//! Centralized Ethereum Event Signature Constants
//!
//! This module provides compile-time validated event signature constants for all DEX protocols.
//! These signatures are the keccak256 hash of the canonical Solidity event definitions and are
//! used for WebSocket event filtering and log parsing.
//!
//! # Why Hardcoded Constants?
//!
//! These values are:
//! - **Deterministic**: Always the same for the same event signature
//! - **Public**: Published in every block on every Ethereum-compatible chain
//! - **Immutable**: Cannot change without breaking existing smart contracts
//! - **Standard**: Part of the ERC/EIP specifications
//!
//! Runtime computation would waste cycles for no benefit, while these constants provide:
//! - Zero-cost lookups in hot paths
//! - Compile-time verification via tests
//! - Single source of truth across all services

use web3::types::H256;

// =============================================================================
// Uniswap V2 Event Signatures
// =============================================================================

/// Uniswap V2 Swap event signature
/// `Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to)`
/// keccak256("Swap(address,uint256,uint256,uint256,uint256,address)")
pub const UNISWAP_V2_SWAP: H256 = H256([
    0xd7, 0x8a, 0xd9, 0x5f, 0xa4, 0x6c, 0x99, 0x4b, 0x65, 0x51, 0xd0, 0xda, 0x85, 0xfc, 0x27, 0x5f,
    0xe6, 0x13, 0xce, 0x37, 0x65, 0x7f, 0xb8, 0xd5, 0xe3, 0xd1, 0x30, 0x84, 0x01, 0x59, 0xd8, 0x22,
]);

/// Uniswap V2 Mint event signature
/// `Mint(address indexed sender, uint256 amount0, uint256 amount1)`
/// keccak256("Mint(address,uint256,uint256)")
pub const UNISWAP_V2_MINT: H256 = H256([
    0x4c, 0x20, 0x9b, 0x5f, 0xc8, 0xad, 0x50, 0x75, 0x8f, 0x13, 0xe2, 0xe1, 0x08, 0x8b, 0xa5, 0x6a,
    0x56, 0x0d, 0xff, 0x69, 0x0a, 0x1c, 0x6f, 0xef, 0x26, 0x39, 0x4f, 0x4c, 0x03, 0x82, 0x1c, 0x4f,
]);

/// Uniswap V2 Burn event signature
/// `Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to)`
/// keccak256("Burn(address,uint256,uint256,address)")
pub const UNISWAP_V2_BURN: H256 = H256([
    0xdc, 0xcd, 0x41, 0x2f, 0x0b, 0x12, 0x52, 0x81, 0x9c, 0xb1, 0xfd, 0x33, 0x0b, 0x93, 0x22, 0x4c,
    0xa4, 0x26, 0x12, 0x89, 0x2b, 0xb3, 0xf4, 0xf7, 0x89, 0x97, 0x6e, 0x6d, 0x81, 0x93, 0x64, 0x96,
]);

/// Uniswap V2 Sync event signature  
/// `Sync(uint112 reserve0, uint112 reserve1)`
/// keccak256("Sync(uint112,uint112)")
pub const UNISWAP_V2_SYNC: H256 = H256([
    0x1c, 0x41, 0x1e, 0x9a, 0x96, 0xe0, 0x71, 0x24, 0x1c, 0x2f, 0x21, 0xf7, 0x72, 0x6b, 0x17, 0xae,
    0x89, 0xe3, 0xca, 0xb4, 0xc7, 0x8b, 0xe5, 0x0e, 0x06, 0x2b, 0x03, 0xa9, 0xff, 0xfb, 0xba, 0xd1,
]);

// =============================================================================
// Uniswap V3 Event Signatures
// =============================================================================

/// Uniswap V3 Swap event signature
/// `Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)`
/// keccak256("Swap(address,address,int256,int256,uint160,uint128,int24)")
pub const UNISWAP_V3_SWAP: H256 = H256([
    0xc4, 0x20, 0x79, 0xf9, 0x4a, 0x63, 0x50, 0xd7, 0xe6, 0x23, 0x5f, 0x29, 0x17, 0x49, 0x24, 0xf9,
    0x28, 0xcc, 0x2a, 0xc8, 0x18, 0xeb, 0x64, 0xfe, 0xd8, 0x00, 0x4e, 0x11, 0x5f, 0xbc, 0xca, 0x67,
]);

/// Uniswap V3 Mint event signature
/// `Mint(address sender, address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)`
/// keccak256("Mint(address,address,int24,int24,uint128,uint256,uint256)")
pub const UNISWAP_V3_MINT: H256 = H256([
    0x7a, 0x53, 0x08, 0x0b, 0xa4, 0x14, 0x15, 0x8b, 0xe7, 0xec, 0x69, 0xb9, 0x87, 0xb5, 0xfb, 0x7d,
    0x07, 0xde, 0xe1, 0x01, 0xfe, 0x85, 0x48, 0x8f, 0x08, 0x53, 0xae, 0x16, 0x23, 0x9d, 0x0b, 0xde,
]);

/// Uniswap V3 Burn event signature
/// `Burn(address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)`
/// keccak256("Burn(address,int24,int24,uint128,uint256,uint256)")
pub const UNISWAP_V3_BURN: H256 = H256([
    0x0c, 0x39, 0x6c, 0xd9, 0x89, 0xa3, 0x9f, 0x44, 0x59, 0xb5, 0xfa, 0x1a, 0xed, 0x6a, 0x9a, 0x8d,
    0xcd, 0xbc, 0x45, 0x90, 0x8a, 0xcf, 0xd6, 0x7e, 0x02, 0x8c, 0xd5, 0x68, 0xda, 0x98, 0x98, 0x2c,
]);

// =============================================================================
// Common ERC-20 Event Signatures
// =============================================================================

/// ERC-20 Transfer event signature
/// `Transfer(address indexed from, address indexed to, uint256 value)`
/// keccak256("Transfer(address,address,uint256)")
pub const ERC20_TRANSFER: H256 = H256([
    0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
    0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
]);

/// ERC-20 Approval event signature
/// `Approval(address indexed owner, address indexed spender, uint256 value)`
/// keccak256("Approval(address,address,uint256)")
pub const ERC20_APPROVAL: H256 = H256([
    0x8c, 0x5b, 0xe1, 0xe5, 0xeb, 0xec, 0x7d, 0x5b, 0xd1, 0x4f, 0x71, 0x42, 0x7d, 0x1e, 0x84, 0xf3,
    0xdd, 0x03, 0x14, 0xc0, 0xf7, 0xb2, 0x29, 0x1e, 0x5b, 0x20, 0x0a, 0xc8, 0xc7, 0xc3, 0xb9, 0x25,
]);

// =============================================================================
// Utility Functions
// =============================================================================

/// Get all DEX event signatures for WebSocket subscription filtering
pub const fn get_all_dex_signatures() -> [H256; 7] {
    [
        UNISWAP_V2_SWAP,
        UNISWAP_V2_MINT, 
        UNISWAP_V2_BURN,
        UNISWAP_V2_SYNC,
        UNISWAP_V3_SWAP,
        UNISWAP_V3_MINT,
        UNISWAP_V3_BURN,
    ]
}

/// Get only swap event signatures for focused trading monitoring
pub const fn get_swap_signatures() -> [H256; 2] {
    [UNISWAP_V2_SWAP, UNISWAP_V3_SWAP]
}

/// Get mint/burn signatures for liquidity monitoring
pub const fn get_liquidity_signatures() -> [H256; 4] {
    [UNISWAP_V2_MINT, UNISWAP_V2_BURN, UNISWAP_V3_MINT, UNISWAP_V3_BURN]
}

/// Convert H256 to hex string for JSON-RPC use
pub fn to_hex_string(hash: H256) -> String {
    format!("0x{:x}", hash)
}

// =============================================================================
// Compile-Time Verification Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::{uniswap_v2, uniswap_v3};

    #[test]
    fn verify_uniswap_v2_swap_signature() {
        let computed = uniswap_v2::swap_event().signature();
        assert_eq!(
            computed, UNISWAP_V2_SWAP,
            "V2 Swap signature mismatch: computed={:x}, constant={:x}",
            computed, UNISWAP_V2_SWAP
        );
    }

    #[test]
    fn verify_uniswap_v2_mint_signature() {
        let computed = uniswap_v2::mint_event().signature();
        assert_eq!(
            computed, UNISWAP_V2_MINT,
            "V2 Mint signature mismatch: computed={:x}, constant={:x}",
            computed, UNISWAP_V2_MINT
        );
    }

    #[test]
    fn verify_uniswap_v2_burn_signature() {
        let computed = uniswap_v2::burn_event().signature();
        assert_eq!(
            computed, UNISWAP_V2_BURN,
            "V2 Burn signature mismatch: computed={:x}, constant={:x}",
            computed, UNISWAP_V2_BURN
        );
    }

    #[test]
    fn verify_uniswap_v2_sync_signature() {
        let computed = uniswap_v2::sync_event().signature();
        assert_eq!(
            computed, UNISWAP_V2_SYNC,
            "V2 Sync signature mismatch: computed={:x}, constant={:x}",
            computed, UNISWAP_V2_SYNC
        );
    }

    #[test]
    fn verify_uniswap_v3_swap_signature() {
        let computed = uniswap_v3::swap_event().signature();
        assert_eq!(
            computed, UNISWAP_V3_SWAP,
            "V3 Swap signature mismatch: computed={:x}, constant={:x}",
            computed, UNISWAP_V3_SWAP
        );
    }

    #[test]
    fn verify_uniswap_v3_mint_signature() {
        let computed = uniswap_v3::mint_event().signature();
        assert_eq!(
            computed, UNISWAP_V3_MINT,
            "V3 Mint signature mismatch: computed={:x}, constant={:x}",
            computed, UNISWAP_V3_MINT
        );
    }

    #[test]
    fn verify_uniswap_v3_burn_signature() {
        let computed = uniswap_v3::burn_event().signature();
        assert_eq!(
            computed, UNISWAP_V3_BURN,
            "V3 Burn signature mismatch: computed={:x}, constant={:x}",
            computed, UNISWAP_V3_BURN
        );
    }

    #[test]
    fn verify_signature_uniqueness() {
        let signatures = get_all_dex_signatures();
        
        // Ensure all signatures are unique
        for (i, sig1) in signatures.iter().enumerate() {
            for (j, sig2) in signatures.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        sig1, sig2,
                        "Duplicate signature found at indices {} and {}: {:x}",
                        i, j, sig1
                    );
                }
            }
        }
    }

    #[test]
    fn verify_string_conversion() {
        let swap_str = to_hex_string(UNISWAP_V3_SWAP);
        assert_eq!(
            swap_str,
            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"
        );
        
        let mint_str = to_hex_string(UNISWAP_V2_MINT);
        assert_eq!(
            mint_str,
            "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f"
        );
    }

    #[test]
    fn verify_constant_arrays() {
        let all_sigs = get_all_dex_signatures();
        assert_eq!(all_sigs.len(), 7);
        
        let swap_sigs = get_swap_signatures();
        assert_eq!(swap_sigs.len(), 2);
        assert_eq!(swap_sigs[0], UNISWAP_V2_SWAP);
        assert_eq!(swap_sigs[1], UNISWAP_V3_SWAP);
        
        let liquidity_sigs = get_liquidity_signatures();
        assert_eq!(liquidity_sigs.len(), 4);
    }

    /// Comprehensive test to ensure all hardcoded signatures in various files
    /// match our centralized constants
    #[test]
    fn verify_hardcoded_signatures_across_codebase() {
        // These are the actual hardcoded strings found in the codebase
        let v2_swap_str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
        let v3_swap_str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
        let v2_mint_str = "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f";
        let v3_mint_str = "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde";
        let v2_burn_str = "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496";

        // Parse and verify they match our constants
        let v2_swap_parsed: H256 = v2_swap_str.parse().expect("Failed to parse V2 swap");
        let v3_swap_parsed: H256 = v3_swap_str.parse().expect("Failed to parse V3 swap");
        let v2_mint_parsed: H256 = v2_mint_str.parse().expect("Failed to parse V2 mint");
        let v3_mint_parsed: H256 = v3_mint_str.parse().expect("Failed to parse V3 mint");
        let v2_burn_parsed: H256 = v2_burn_str.parse().expect("Failed to parse V2 burn");

        assert_eq!(v2_swap_parsed, UNISWAP_V2_SWAP, "V2 Swap hardcoded signature mismatch");
        assert_eq!(v3_swap_parsed, UNISWAP_V3_SWAP, "V3 Swap hardcoded signature mismatch");
        assert_eq!(v2_mint_parsed, UNISWAP_V2_MINT, "V2 Mint hardcoded signature mismatch");
        assert_eq!(v3_mint_parsed, UNISWAP_V3_MINT, "V3 Mint hardcoded signature mismatch");
        assert_eq!(v2_burn_parsed, UNISWAP_V2_BURN, "V2 Burn hardcoded signature mismatch");
    }

    /// Test that verifies the mathematical relationship between event signatures
    /// This ensures the signatures follow expected patterns
    #[test]
    fn verify_signature_mathematical_properties() {
        // All signatures should be non-zero
        for sig in get_all_dex_signatures().iter() {
            assert_ne!(*sig, H256::zero(), "Signature cannot be zero");
        }

        // Swap signatures should be different for V2 vs V3
        assert_ne!(UNISWAP_V2_SWAP, UNISWAP_V3_SWAP);
        
        // Same event types should be different between protocols
        assert_ne!(UNISWAP_V2_MINT, UNISWAP_V3_MINT);
        assert_ne!(UNISWAP_V2_BURN, UNISWAP_V3_BURN);
    }
}