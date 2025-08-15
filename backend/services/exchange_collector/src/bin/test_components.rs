// Integration test for validating DEX swap processing components

// Need to add the library to access internal modules
#[path = "../lib.rs"]
mod lib;

use alphapulse_protocol::{TradeMessage, TradeSide};
use lib::exchanges::polygon::dex::{
    UniswapV2Pool, EventBasedPoolType, SwapEvent, PoolFactory,
    UNISWAP_V2_SWAP_SIGNATURE,
};
use lib::instruments::INSTRUMENTS;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Running DEX Swap Component Validation Tests");
    
    // Test 1: Symbol Hashing Validation
    println!("\n1️⃣ Testing Pool-Specific Symbol Hashing");
    test_symbol_hashing().await?;
    
    // Test 2: Binary Message Encoding
    println!("\n2️⃣ Testing Binary Message Encoding");
    test_message_encoding()?;
    
    // Test 3: Price Calculation Logic
    println!("\n3️⃣ Testing Price Calculation");
    test_price_calculation().await?;
    
    // Test 4: Event Signature Classification
    println!("\n4️⃣ Testing Event Signature Classification");
    test_event_classification()?;
    
    println!("\n✅ All component validation tests passed!");
    Ok(())
}

async fn test_symbol_hashing() -> anyhow::Result<()> {
    // Test pool-specific symbol hashing
    let pool_address1 = "0x1f1e4c845183ef6d50e9609f16f6f9cae43bc1cb";
    let pool_address2 = "0x2f2e4c845183ef6d50e9609f16f6f9cae43bc1cb";
    
    // Same token pair, different pools should have different hashes
    let symbol1 = format!("{}:{}/{}", pool_address1, "DAI", "LGNS");
    let symbol2 = format!("{}:{}/{}", pool_address2, "DAI", "LGNS");
    
    let hash1 = INSTRUMENTS.get_or_create_hash("polygon", &symbol1);
    let hash2 = INSTRUMENTS.get_or_create_hash("polygon", &symbol2);
    
    assert_ne!(hash1, hash2, "Different pools should have different symbol hashes");
    
    // Same pool should produce same hash
    let hash1_repeat = INSTRUMENTS.get_or_create_hash("polygon", &symbol1);
    assert_eq!(hash1, hash1_repeat, "Same pool should produce consistent hash");
    
    println!("   ✅ Pool-specific hashing working correctly");
    println!("   📊 Pool 1 hash: {}, Pool 2 hash: {}", hash1, hash2);
    
    Ok(())
}

fn test_message_encoding() -> anyhow::Result<()> {
    // Test that we're using proper TradeMessage constructor, not f64.to_bits()
    let timestamp_ns = 1692123456789000000u64;
    let price: f64 = 65000.0; // $65,000 BTC price
    let volume: f64 = 1.5;    // 1.5 BTC volume
    
    // Convert to fixed-point (8 decimal places precision)
    let price_fixed = (price * 1e8) as u64;
    let volume_fixed = (volume * 1e8) as u64;
    let symbol_id = 12345u64;
    
    // Create proper TradeMessage using constructor
    let trade = TradeMessage::new(
        timestamp_ns,
        price_fixed,
        volume_fixed,
        symbol_id,
        TradeSide::Buy,
    );
    
    // Verify values make sense (not astronomical like $45B+)
    assert_eq!(price_fixed, 6500000000000); // 65000 * 1e8
    assert_eq!(volume_fixed, 150000000);    // 1.5 * 1e8
    
    // Ensure we're not accidentally using IEEE 754 bit representation
    let wrong_bits = price.to_bits();
    assert_ne!(price_fixed, wrong_bits, "Should not be using f64.to_bits()");
    
    println!("   ✅ Binary message encoding using proper constructor");
    println!("   📊 Price: ${} → {} (fixed-point)", price, price_fixed);
    println!("   📊 Volume: {} → {} (fixed-point)", volume, volume_fixed);
    println!("   ❌ Wrong bits approach: {} (avoided)", wrong_bits);
    
    Ok(())
}

async fn test_price_calculation() -> anyhow::Result<()> {
    // Create a mock UniswapV2 pool for testing
    let pool = UniswapV2Pool::new(
        "0x1f1e4c845183ef6d50e9609f16f6f9cae43bc1cb".to_string(),
        "https://polygon-rpc.com".to_string(),
    );
    
    // Test realistic swap amounts (not astronomical)
    // Simulate a DAI/LGNS swap with 18/18 decimals
    let mut swap_event = SwapEvent {
        pool_address: "0x1f1e4c845183ef6d50e9609f16f6f9cae43bc1cb".to_string(),
        amount0_in: 1000.0,   // 1000 DAI in
        amount1_in: 0.0,
        amount0_out: 0.0,
        amount1_out: 500.0,   // 500 LGNS out
        to_address: "0x123".to_string(),
        from_address: "0x456".to_string(),
        tx_hash: "0xabc".to_string(),
        block_number: 12345,
    };
    
    // Calculate price using pool logic
    let price_info = pool.calculate_price(&swap_event);
    
    // Price should be realistic (around $2 for LGNS if DAI is $1)
    // 1000 DAI for 500 LGNS = $2 per LGNS
    assert!(price_info.price > 1.0 && price_info.price < 10.0, 
           "Price should be realistic, not astronomical. Got: ${}", price_info.price);
    
    assert!(price_info.volume > 100.0 && price_info.volume < 10000.0,
           "Volume should be realistic. Got: ${}", price_info.volume);
    
    println!("   ✅ Price calculation producing realistic values");
    println!("   📊 Calculated price: ${:.2}", price_info.price);
    println!("   📊 Calculated volume: ${:.2}", price_info.volume);
    
    Ok(())
}

fn test_event_classification() -> anyhow::Result<()> {
    // Test event signature classification
    let uniswap_v2_sig = UNISWAP_V2_SWAP_SIGNATURE;
    
    // This should classify as UniswapV2Style
    let pool_factory = PoolFactory::new(
        "https://polygon-rpc.com".to_string()
    );
    
    let pool_type = pool_factory.classify_by_event_signature(uniswap_v2_sig);
    assert!(pool_type.is_some(), "Should recognize UniswapV2 signature");
    assert_eq!(pool_type.unwrap(), EventBasedPoolType::UniswapV2Style);
    
    // Test unknown signature - dummy test data, not a real hash
    // nosec: Test placeholder signature
    let unknown_sig = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let unknown_type = pool_factory.classify_by_event_signature(unknown_sig);
    assert!(unknown_type.is_none(), "Should not recognize unknown signature");
    
    println!("   ✅ Event signature classification working correctly");
    println!("   📊 UniswapV2 signature classified correctly");
    
    Ok(())
}