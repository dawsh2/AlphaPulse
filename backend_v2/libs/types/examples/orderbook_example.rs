//! # OrderBook TLV Implementation Example
//!
//! Demonstrates complete OrderBook TLV implementation following AlphaPulse Protocol V2 patterns:
//! - Production-ready precision handling (8-decimal for traditional exchanges, native for DEX)
//! - Zero-copy serialization with DynamicPayload pattern
//! - Comprehensive validation with integrity checks
//! - TLVMessageBuilder integration for domain routing
//! - Performance-optimized data structures

use alphapulse_types::protocol::{
    identifiers::instrument::{AssetType, TokenAddress},
    tlv::{OrderBookTLV, OrderLevel, TLVMessageBuilder, TLVType},
    InstrumentId, RelayDomain, SourceType, VenueId,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 AlphaPulse OrderBook TLV Implementation Demo");
    println!("================================================");

    // 1. Create InstrumentId for BTC/USD on Coinbase (8-decimal precision)
    let btc_usd_instrument = InstrumentId::build_traditional_spot(
        VenueId::Coinbase,
        AssetType::Traditional,
        "BTC/USD".as_bytes(),
    )?;

    // 2. Initialize empty order book with 8-decimal precision (100,000,000)
    let timestamp_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos() as u64;

    let mut order_book = OrderBookTLV::from_instrument(
        VenueId::Coinbase,
        btc_usd_instrument,
        timestamp_ns,
        12345,          // sequence number
        100_000_000i64, // 8-decimal precision for traditional exchange
    );

    println!(
        "✅ Created OrderBook for {}",
        String::from_utf8_lossy(&btc_usd_instrument.asset_id.to_le_bytes())
    );

    // 3. Add bid levels (price in 8-decimal fixed-point: $45,000.00 = 4500000000000)
    order_book.add_bid(4500000000000i64, 150000000i64, 3); // $45,000.00, 1.5 BTC, 3 orders
    order_book.add_bid(4499900000000i64, 250000000i64, 5); // $44,999.00, 2.5 BTC, 5 orders
    order_book.add_bid(4499500000000i64, 100000000i64, 2); // $44,995.00, 1.0 BTC, 2 orders

    // 4. Add ask levels
    order_book.add_ask(4500100000000i64, 120000000i64, 2); // $45,001.00, 1.2 BTC, 2 orders
    order_book.add_ask(4500500000000i64, 300000000i64, 7); // $45,005.00, 3.0 BTC, 7 orders
    order_book.add_ask(4501000000000i64, 80000000i64, 1); // $45,010.00, 0.8 BTC, 1 order

    println!("📊 Added bid/ask levels:");

    // 5. Display order book state
    println!(
        "Best Bid: ${:.2} (Size: {:.3} BTC)",
        order_book
            .best_bid()
            .unwrap()
            .price_decimal(order_book.precision_factor),
        order_book
            .best_bid()
            .unwrap()
            .size_decimal(order_book.precision_factor)
    );

    println!(
        "Best Ask: ${:.2} (Size: {:.3} BTC)",
        order_book
            .best_ask()
            .unwrap()
            .price_decimal(order_book.precision_factor),
        order_book
            .best_ask()
            .unwrap()
            .size_decimal(order_book.precision_factor)
    );

    println!("Spread: {} basis points", order_book.spread_bps().unwrap());

    // 6. Build Protocol V2 message with proper domain routing
    let order_book_bytes = order_book.to_bytes()?;
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::CoinbaseCollector)
        .add_tlv_raw(TLVType::OrderBook as u8, &order_book_bytes)
        .build();

    println!("📦 Built TLV Message:");
    println!("  - Size: {} bytes", message.as_bytes().len());
    println!("  - Domain: MarketData → routes to strategies/dashboard");
    println!("  - Payload Size: {} bytes", order_book.payload_size());

    // 7. Demonstrate serialization/deserialization roundtrip
    let serialized = order_book.to_bytes()?;
    let deserialized = OrderBookTLV::from_bytes(&serialized)?;

    println!("🔄 Serialization Roundtrip:");
    println!(
        "  - Original levels: {} bids, {} asks",
        order_book.bids.len(),
        order_book.asks.len()
    );
    println!(
        "  - Deserialized levels: {} bids, {} asks",
        deserialized.bids.len(),
        deserialized.asks.len()
    );
    println!(
        "  - Spread preserved: {} bps",
        deserialized.spread_bps().unwrap()
    );

    // 8. Show validation in action
    println!("✅ Validation passes: order book integrity maintained");
    deserialized.validate()?;

    // 9. DEX example with native token precision
    println!("\n🏦 DEX Example (Native Token Precision):");

    let weth_usdc_pool = InstrumentId::build_dex_pool(
        VenueId::Uniswap,
        TokenAddress::from_hex("0xa0b86a33e6551006d5e1f17b45f7e9c7c4b5f0e2")?, // WETH
        TokenAddress::from_hex("0xa0b86a33e6551006d5e1f17b45f7e9c7c4b5f0e3")?, // USDC
        3000,                                                                  // 0.3% fee tier
    )?;

    let mut dex_order_book = OrderBookTLV::from_instrument(
        VenueId::Uniswap,
        weth_usdc_pool,
        timestamp_ns,
        67890,
        1i64, // Native precision (no scaling)
    );

    // Add levels with native token precision
    // WETH: 18 decimals, USDC: 6 decimals
    dex_order_book.add_bid(4500000000i64, 1500000000000000000i64, 0); // $4,500 USDC, 1.5 WETH
    dex_order_book.add_ask(4501000000i64, 800000000000000000i64, 0); // $4,501 USDC, 0.8 WETH

    println!("  - DEX Pool: WETH/USDC 0.3%");
    println!(
        "  - Best Bid: {} USDC per WETH",
        dex_order_book.best_bid().unwrap().price
    );
    println!(
        "  - Best Ask: {} USDC per WETH",
        dex_order_book.best_ask().unwrap().price
    );

    println!("\n🎯 Implementation Features Demonstrated:");
    println!("  ✅ Production-ready precision handling");
    println!("  ✅ Zero-copy serialization with validation");
    println!("  ✅ Automatic ordering maintenance");
    println!("  ✅ Comprehensive integrity checks");
    println!("  ✅ Protocol V2 domain routing");
    println!("  ✅ Both traditional exchange and DEX support");
    println!("  ✅ Performance-optimized data structures");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_precision_handling() -> Result<(), Box<dyn std::error::Error>> {
        // Test traditional exchange precision (8-decimal)
        let instrument = InstrumentId::build_traditional_spot(
            VenueId::Binance,
            AssetType::Traditional,
            "ETH/USD".as_bytes(),
        )?;

        let mut book = OrderBookTLV::from_instrument(
            VenueId::Binance,
            instrument,
            1234567890,
            1,
            100_000_000i64, // 8-decimal precision
        );

        // Add level: $3,000.50 = 300050000000 (8 decimals)
        book.add_bid(300050000000i64, 200000000i64, 1);

        let bid = book.best_bid().unwrap();
        assert_eq!(bid.price_decimal(book.precision_factor), 3000.5);
        assert_eq!(bid.size_decimal(book.precision_factor), 2.0);

        Ok(())
    }

    #[test]
    fn test_orderbook_validation() -> Result<(), Box<dyn std::error::Error>> {
        let instrument = InstrumentId::build_traditional_spot(
            VenueId::Kraken,
            AssetType::Traditional,
            "BTC/USD".as_bytes(),
        )?;

        let mut book = OrderBookTLV::from_instrument(
            VenueId::Kraken,
            instrument,
            1234567890,
            1,
            100_000_000i64,
        );

        // Add valid levels
        book.add_bid(4500000000000i64, 100000000i64, 1);
        book.add_ask(4500100000000i64, 100000000i64, 1);

        // Validation should pass
        assert!(book.validate().is_ok());

        // Test spread calculation
        assert!(book.spread_bps().unwrap() > 0);

        Ok(())
    }

    #[test]
    fn test_dex_orderbook_native_precision() -> Result<(), Box<dyn std::error::Error>> {
        // Test DEX with native token precision
        let pool = InstrumentId::build_dex_pool(
            VenueId::Uniswap,
            TokenAddress::from_hex("0xa0b86a33e6551006d5e1f17b45f7e9c7c4b5f0e2")?,
            TokenAddress::from_hex("0xa0b86a33e6551006d5e1f17b45f7e9c7c4b5f0e3")?,
            500, // 0.05% fee tier
        )?;

        let mut dex_book = OrderBookTLV::from_instrument(
            VenueId::Uniswap,
            pool,
            1234567890,
            1,
            1i64, // Native precision (no scaling)
        );

        // Native precision values
        dex_book.add_bid(3000000000i64, 1000000000000000000i64, 0); // 3000 USDC, 1 WETH
        dex_book.add_ask(3001000000i64, 500000000000000000i64, 0); // 3001 USDC, 0.5 WETH

        assert_eq!(dex_book.best_bid().unwrap().price, 3000000000i64);
        assert_eq!(dex_book.best_ask().unwrap().price, 3001000000i64);

        Ok(())
    }
}
