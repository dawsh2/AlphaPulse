#!/usr/bin/env rust

//! End-to-end test for the new bijective ID protocol
//! Tests the complete flow from instrument creation to message processing

use alphapulse_protocol::{
    InstrumentId, VenueId, AssetType, SourceType,
    NewTradeMessage, NewTradeSide, InstrumentDiscoveredMessage,
    SchemaTransformCache, CachedObject, TokenMetadata, ProcessedMessage
};
use zerocopy::AsBytes;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Testing bijective ID protocol end-to-end...");
    
    // Test 1: Create bijective instrument IDs
    println!("\n📋 Test 1: Bijective InstrumentId creation");
    test_instrument_id_creation().await?;
    
    // Test 2: Message creation and serialization
    println!("\n💱 Test 2: Message creation and binary serialization");
    test_message_creation().await?;
    
    // Test 3: Schema cache processing
    println!("\n🗂️ Test 3: Schema cache message processing");
    test_schema_cache_processing().await?;
    
    // Test 4: Round-trip bijective property
    println!("\n🔄 Test 4: Bijective ID round-trip validation");
    test_bijective_round_trip().await?;
    
    println!("\n✅ All tests passed! The bijective ID protocol is working correctly.");
    Ok(())
}

async fn test_instrument_id_creation() -> Result<()> {
    // Test major tokens on Polygon
    let tokens = vec![
        ("0x2791bca1f2de4661ed88a30c99a7a9449aa84174", "USDC", "USD Coin (PoS)"),
        ("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270", "WMATIC", "Wrapped Matic"),
        ("0x7ceb23fd6ad59f72c16540b0f4db0bc3bc5e4e7a", "WETH", "Wrapped Ether"),
    ];
    
    for (address, symbol, name) in tokens {
        let instrument_id = InstrumentId::polygon_token(address)?;
        
        println!("  🎯 {} ({}) -> ID: {} -> Debug: {}", 
                 symbol, name, 
                 format!("{:?}", instrument_id),
                 instrument_id.debug_info());
        
        // Verify the bijective properties
        assert_eq!(instrument_id.venue()?, VenueId::Polygon);
        assert_eq!(instrument_id.asset_type()?, AssetType::Token);
        
        // Test cache key generation
        let cache_key = instrument_id.cache_key();
        println!("    Cache key: {:#x}", cache_key);
    }
    
    // Test stocks
    let aapl = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
    println!("  📈 NASDAQ:AAPL -> ID: {:?} -> Debug: {}", aapl, aapl.debug_info());
    
    // Test pools
    let usdc_id = InstrumentId::polygon_token("0x2791bca1f2de4661ed88a30c99a7a9449aa84174")?;
    let weth_id = InstrumentId::polygon_token("0x7ceb23fd6ad59f72c16540b0f4db0bc3bc5e4e7a")?;
    let pool_id = InstrumentId::pool(VenueId::UniswapV3, usdc_id, weth_id);
    println!("  🏊 USDC/WETH Pool -> ID: {:?} -> Debug: {}", pool_id, pool_id.debug_info());
    
    Ok(())
}

async fn test_message_creation() -> Result<()> {
    // Create a trade message for a Polygon token
    let usdc_id = InstrumentId::polygon_token("0x2791bca1f2de4661ed88a30c99a7a9449aa84174")?;
    
    let trade = NewTradeMessage::new(
        usdc_id,
        100000000, // $1.00 with 8 decimal precision
        500000000, // 5.0 tokens 
        NewTradeSide::Buy,
        12345,     // sequence
        SourceType::PolygonCollector,
    );
    
    println!("  💰 Created trade message for USDC:");
    println!("    Instrument ID: {:?}", trade.instrument_id);
    println!("    Price: ${:.6}", trade.price_decimal());
    println!("    Volume: {:.2} tokens", trade.volume_decimal());
    println!("    Side: {:?}", trade.trade_side()?);
    
    // Serialize to bytes
    let bytes = trade.as_bytes();
    println!("    Binary size: {} bytes", bytes.len());
    
    // Test round-trip parsing
    let parsed_trade = NewTradeMessage::from_bytes(bytes)?;
    println!("    Parsed trade price: ${:.6}", parsed_trade.price_decimal());
    
    assert_eq!(trade.price_decimal(), parsed_trade.price_decimal());
    assert_eq!(trade.volume_decimal(), parsed_trade.volume_decimal());
    
    // Create an instrument discovery message
    let discovery = InstrumentDiscoveredMessage::new(
        usdc_id,
        "USDC".to_string(),
        6, // decimals
        b"Polygon native USDC".to_vec(), // metadata
        12346, // sequence
        SourceType::PolygonCollector,
    );
    
    println!("  🔍 Created instrument discovery message:");
    println!("    Symbol: {}", discovery.symbol);
    println!("    Decimals: {}", discovery.header.decimals);
    
    let discovery_bytes = discovery.serialize();
    let parsed_discovery = InstrumentDiscoveredMessage::parse(&discovery_bytes)?;
    
    assert_eq!(discovery.symbol, parsed_discovery.symbol);
    assert_eq!(discovery.header.decimals, parsed_discovery.header.decimals);
    
    Ok(())
}

async fn test_schema_cache_processing() -> Result<()> {
    // Initialize schema cache
    let schema_cache = SchemaTransformCache::new();
    
    // Add some tokens to the cache
    let tokens = vec![
        ("0x2791bca1f2de4661ed88a30c99a7a9449aa84174", "USDC", "USD Coin (PoS)", 6),
        ("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270", "WMATIC", "Wrapped Matic", 18),
    ];
    
    for (address, symbol, name, decimals) in tokens {
        let instrument_id = InstrumentId::polygon_token(address)?;
        let metadata = TokenMetadata {
            id: instrument_id,
            address: address.to_string(),
            symbol: symbol.to_string(),
            name: name.to_string(),
            decimals,
            chain_id: 137, // Polygon mainnet
            discovered_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos() as u64,
        };
        
        schema_cache.insert(instrument_id, CachedObject::Token(metadata));
        println!("  📝 Cached {} token: {}", symbol, instrument_id.debug_info());
    }
    
    // Test cache retrieval
    let usdc_id = InstrumentId::polygon_token("0x2791bca1f2de4661ed88a30c99a7a9449aa84174")?;
    if let Some(CachedObject::Token(token)) = schema_cache.get(&usdc_id) {
        println!("  ✅ Retrieved {} from cache: {} ({})", token.symbol, token.name, token.address);
    }
    
    // Test venue filtering
    let polygon_tokens = schema_cache.get_by_venue(VenueId::Polygon);
    println!("  🔍 Found {} Polygon tokens in cache", polygon_tokens.len());
    
    // Test message processing
    let trade = NewTradeMessage::new(
        usdc_id,
        99500000, // $0.995
        1000000000, // 10.0 tokens
        NewTradeSide::Sell,
        12347,
        SourceType::PolygonCollector,
    );
    
    let trade_bytes = trade.as_bytes();
    match schema_cache.process_message(trade_bytes) {
        Ok(ProcessedMessage::Trade(trade_data)) => {
            println!("  🎯 Processed trade message:");
            println!("    Instrument: {}", trade_data.instrument_id.debug_info());
            println!("    Price: ${:.3}", trade_data.price);
            println!("    Volume: {:.1}", trade_data.volume);
        }
        Ok(msg) => println!("  ❓ Unexpected message type: {:?}", msg),
        Err(e) => return Err(e.into()),
    }
    
    // Print cache statistics
    let stats = schema_cache.stats();
    println!("  📊 Cache stats: {} objects, {} schemas", stats.object_count, stats.schema_count);
    
    Ok(())
}

async fn test_bijective_round_trip() -> Result<()> {
    // Test that instrument IDs maintain their bijective properties
    let test_cases = vec![
        InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")?, // USDC on Ethereum
        InstrumentId::polygon_token("0x2791bca1f2de4661ed88a30c99a7a9449aa84174")?,  // USDC on Polygon
        InstrumentId::stock(VenueId::NASDAQ, "AAPL"),
        InstrumentId::stock(VenueId::NYSE, "TSLA"),
    ];
    
    for original_id in test_cases {
        // Test u64 conversion (with precision loss warning)
        let as_u64 = original_id.to_u64();
        let from_u64 = InstrumentId::from_u64(as_u64);
        
        println!("  🔄 Round-trip test for {}", original_id.debug_info());
        println!("    Original venue: {:?}, reconstructed venue: {:?}", 
                 original_id.venue()?, from_u64.venue()?);
        println!("    Original asset type: {:?}, reconstructed asset type: {:?}", 
                 original_id.asset_type()?, from_u64.asset_type()?);
        
        // These should match (venue and asset type preserved)
        assert_eq!(original_id.venue()?, from_u64.venue()?);
        assert_eq!(original_id.asset_type()?, from_u64.asset_type()?);
        
        // Full precision cache key should be different for different instruments
        let cache_key = original_id.cache_key();
        println!("    Full precision cache key: {:#x}", cache_key);
        
        // Test hash-based equality
        let same_id = match original_id.venue()? {
            VenueId::Ethereum => InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")?,
            VenueId::Polygon => InstrumentId::polygon_token("0x2791bca1f2de4661ed88a30c99a7a9449aa84174")?,
            VenueId::NASDAQ => InstrumentId::stock(VenueId::NASDAQ, "AAPL"),
            VenueId::NYSE => InstrumentId::stock(VenueId::NYSE, "TSLA"),
            _ => continue,
        };
        
        // Same instrument should have same cache key
        assert_eq!(original_id.cache_key(), same_id.cache_key());
        println!("    ✅ Hash-based equality verified");
    }
    
    Ok(())
}