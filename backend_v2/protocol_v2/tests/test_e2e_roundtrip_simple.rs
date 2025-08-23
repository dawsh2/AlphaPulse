//! Simple standalone test for e2e roundtrip equality
//! Can be run with: cargo test --manifest-path protocol_v2/Cargo.toml --test ../test_e2e_roundtrip_simple

use alphapulse_protocol_v2::{tlv::market_data::PoolSwapTLV, VenueId};

#[tokio::test]
async fn test_pool_swap_tlv_roundtrip() {
    println!("\n🔍 TESTING POOL SWAP TLV ROUNDTRIP EQUALITY\n");

    // Create a realistic PoolSwapTLV with large amounts (exceeding i64::MAX)
    let original_swap = PoolSwapTLV {
        venue: VenueId::QuickSwap,
        pool_address: [
            0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 0x51, 0x51, 0x31, 0xf6,
            0x47, 0xd7, 0x26, 0xf5, 0x06, 0x08,
        ], // Real QuickSwap pool
        token_in_addr: [1; 20],                     // Token0 address
        token_out_addr: [2; 20],                    // Token1 address
        amount_in: 343_806_693_284_695_519_153u128, // Large amount > i64::MAX
        amount_out: 497_159_402_058_990_849u128,    // Another large amount
        amount_in_decimals: 18,                     // WMATIC decimals
        amount_out_decimals: 6,                     // USDC decimals
        sqrt_price_x96_after: 79228162514264337593543950336u128, // √1 in Q96
        tick_after: 0,                              // V2 pools don't have ticks
        liquidity_after: 1_000_000_000_000_000_000u128,
        timestamp_ns: 1700000000000000000u64,
        block_number: 52_000_000u64,
    };

    println!("📦 ORIGINAL TLV:");
    println!("  venue: {:?}", original_swap.venue);
    println!(
        "  pool_address: 0x{}",
        hex::encode(&original_swap.pool_address)
    );
    println!("  amount_in: {}", original_swap.amount_in);
    println!("  amount_out: {}", original_swap.amount_out);
    println!("  amount_in_decimals: {}", original_swap.amount_in_decimals);
    println!(
        "  amount_out_decimals: {}",
        original_swap.amount_out_decimals
    );
    println!(
        "  sqrt_price_x96_after: {}",
        original_swap.sqrt_price_x96_after
    );
    println!("  tick_after: {}", original_swap.tick_after);
    println!("  liquidity_after: {}", original_swap.liquidity_after);
    println!("  timestamp_ns: {}", original_swap.timestamp_ns);
    println!("  block_number: {}", original_swap.block_number);

    // Check large value handling
    println!("\n⚠️ LARGE VALUE VALIDATION:");
    println!("  i64::MAX = {}", i64::MAX);
    if original_swap.amount_in > i64::MAX as u128 {
        println!(
            "  ✅ amount_in ({}) exceeds i64::MAX by {}x",
            original_swap.amount_in,
            original_swap.amount_in / (i64::MAX as u128)
        );
    }
    if original_swap.amount_out > i64::MAX as u128 {
        println!(
            "  ✅ amount_out ({}) exceeds i64::MAX by {}x",
            original_swap.amount_out,
            original_swap.amount_out / (i64::MAX as u128)
        );
    }

    // STEP 1: Serialize to binary
    println!("\n🔄 STEP 1: SERIALIZING TO BINARY");
    let serialized_bytes = original_swap.to_bytes();
    println!("  Serialized to {} bytes", serialized_bytes.len());
    println!(
        "  First 32 bytes: {}",
        hex::encode(&serialized_bytes[..32.min(serialized_bytes.len())])
    );
    if serialized_bytes.len() > 32 {
        println!("  ... (truncated)");
    }

    // STEP 2: Deserialize from binary
    println!("\n🔄 STEP 2: DESERIALIZING FROM BINARY");
    let deserialized_swap = match PoolSwapTLV::from_bytes(&serialized_bytes) {
        Ok(swap) => swap,
        Err(e) => {
            println!("  ❌ Failed to deserialize: {}", e);
            panic!("Deserialization failed");
        }
    };

    println!("  Deserialized successfully!");

    // STEP 3: Deep equality verification
    println!("\n🔍 STEP 3: DEEP EQUALITY VERIFICATION");

    let mut all_equal = true;

    // Check each field individually
    if original_swap.venue != deserialized_swap.venue {
        println!(
            "  ❌ venue mismatch: {:?} → {:?}",
            original_swap.venue, deserialized_swap.venue
        );
        all_equal = false;
    } else {
        println!("  ✅ venue: {:?} (identical)", original_swap.venue);
    }

    if original_swap.pool_address != deserialized_swap.pool_address {
        println!("  ❌ pool_address mismatch");
        all_equal = false;
    } else {
        println!("  ✅ pool_address: identical");
    }

    if original_swap.amount_in != deserialized_swap.amount_in {
        println!(
            "  ❌ amount_in mismatch: {} → {}",
            original_swap.amount_in, deserialized_swap.amount_in
        );
        all_equal = false;
    } else {
        println!("  ✅ amount_in: {} (identical)", original_swap.amount_in);
    }

    if original_swap.amount_out != deserialized_swap.amount_out {
        println!(
            "  ❌ amount_out mismatch: {} → {}",
            original_swap.amount_out, deserialized_swap.amount_out
        );
        all_equal = false;
    } else {
        println!("  ✅ amount_out: {} (identical)", original_swap.amount_out);
    }

    if original_swap.amount_in_decimals != deserialized_swap.amount_in_decimals {
        println!(
            "  ❌ amount_in_decimals mismatch: {} → {}",
            original_swap.amount_in_decimals, deserialized_swap.amount_in_decimals
        );
        all_equal = false;
    } else {
        println!(
            "  ✅ amount_in_decimals: {} (identical)",
            original_swap.amount_in_decimals
        );
    }

    if original_swap.amount_out_decimals != deserialized_swap.amount_out_decimals {
        println!(
            "  ❌ amount_out_decimals mismatch: {} → {}",
            original_swap.amount_out_decimals, deserialized_swap.amount_out_decimals
        );
        all_equal = false;
    } else {
        println!(
            "  ✅ amount_out_decimals: {} (identical)",
            original_swap.amount_out_decimals
        );
    }

    if original_swap.sqrt_price_x96_after != deserialized_swap.sqrt_price_x96_after {
        println!(
            "  ❌ sqrt_price_x96_after mismatch: {} → {}",
            original_swap.sqrt_price_x96_after, deserialized_swap.sqrt_price_x96_after
        );
        all_equal = false;
    } else {
        println!("  ✅ sqrt_price_x96_after: identical");
    }

    if original_swap.tick_after != deserialized_swap.tick_after {
        println!(
            "  ❌ tick_after mismatch: {} → {}",
            original_swap.tick_after, deserialized_swap.tick_after
        );
        all_equal = false;
    } else {
        println!("  ✅ tick_after: {} (identical)", original_swap.tick_after);
    }

    if original_swap.liquidity_after != deserialized_swap.liquidity_after {
        println!(
            "  ❌ liquidity_after mismatch: {} → {}",
            original_swap.liquidity_after, deserialized_swap.liquidity_after
        );
        all_equal = false;
    } else {
        println!("  ✅ liquidity_after: identical");
    }

    if original_swap.timestamp_ns != deserialized_swap.timestamp_ns {
        println!(
            "  ❌ timestamp_ns mismatch: {} → {}",
            original_swap.timestamp_ns, deserialized_swap.timestamp_ns
        );
        all_equal = false;
    } else {
        println!("  ✅ timestamp_ns: identical");
    }

    if original_swap.block_number != deserialized_swap.block_number {
        println!(
            "  ❌ block_number mismatch: {} → {}",
            original_swap.block_number, deserialized_swap.block_number
        );
        all_equal = false;
    } else {
        println!(
            "  ✅ block_number: {} (identical)",
            original_swap.block_number
        );
    }

    // STEP 4: Final result
    println!("\n🏁 FINAL RESULT:");
    if all_equal {
        println!("  🎉 PERFECT E2E ROUNDTRIP EQUALITY!");
        println!("     ✅ Semantic data preserved");
        println!("     ✅ Serialization preserves data");
        println!("     ✅ Deserialization identical");
        println!("     ✅ No precision loss");
        println!("     🏆 Successfully handled values exceeding i64::MAX!");

        // Test structural equality too
        assert_eq!(
            original_swap, deserialized_swap,
            "Structural equality failed"
        );
        println!("     ✅ Structural equality confirmed");
    } else {
        println!("  ❌ E2E EQUALITY TEST FAILED");
        println!("     Data corruption during serialization/deserialization");
        panic!("E2E roundtrip equality test failed");
    }

    println!("\n{}", "=".repeat(80));
    println!("✅ E2E ROUNDTRIP EQUALITY TEST COMPLETED SUCCESSFULLY");
    println!("{}", "=".repeat(80));
}
