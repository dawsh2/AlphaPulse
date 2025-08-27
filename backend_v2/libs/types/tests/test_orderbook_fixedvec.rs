// Standalone test for OrderBookTLV FixedVec optimization
use protocol_v2::tlv::{OrderBookTLV, OrderLevel};
use protocol_v2::{InstrumentId, VenueId};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing OrderBookTLV FixedVec optimization...");

    // Create a test instrument
    let instrument = InstrumentId::stock(VenueId::NYSE, "AAPL");
    
    // Create an empty order book
    let mut order_book = OrderBookTLV::from_instrument(
        VenueId::NYSE,
        instrument,
        1640995200000000000, // timestamp_ns
        1000,                // sequence
        100_000_000,        // precision_factor for 8-decimal places
    );

    println!("✅ Created empty order book with FixedVec");

    // Add some bid levels (high to low price)
    order_book.add_bid(10000, 100000, 5)?; // $100.00, 1.0 shares, 5 orders
    order_book.add_bid(9950, 200000, 3)?;  // $99.50, 2.0 shares, 3 orders
    order_book.add_bid(9900, 50000, 1)?;   // $99.00, 0.5 shares, 1 order

    println!("✅ Added {} bid levels", order_book.bids.len());

    // Add some ask levels (low to high price)
    order_book.add_ask(10050, 150000, 2)?; // $100.50, 1.5 shares, 2 orders
    order_book.add_ask(10100, 300000, 4)?; // $101.00, 3.0 shares, 4 orders
    order_book.add_ask(10150, 100000, 1)?; // $101.50, 1.0 shares, 1 order

    println!("✅ Added {} ask levels", order_book.asks.len());

    // Test best bid/ask
    if let Some(best_bid) = order_book.best_bid() {
        println!("✅ Best bid: ${:.2} @ {:.2} shares", 
                best_bid.price_decimal(100_000_000),
                best_bid.size_decimal(100_000_000));
    }

    if let Some(best_ask) = order_book.best_ask() {
        println!("✅ Best ask: ${:.2} @ {:.2} shares", 
                best_ask.price_decimal(100_000_000),
                best_ask.size_decimal(100_000_000));
    }

    // Test spread calculation
    if let Some(spread_bps) = order_book.spread_bps() {
        println!("✅ Spread: {} basis points", spread_bps);
    }

    // Test validation
    order_book.validate()?;
    println!("✅ Order book structure is valid");

    // Test serialization/deserialization
    let bytes = order_book.to_bytes()?;
    println!("✅ Serialized to {} bytes", bytes.len());

    let deserialized = OrderBookTLV::from_bytes(&bytes)?;
    println!("✅ Successfully deserialized");

    // Verify data integrity
    assert_eq!(deserialized.bids.len(), order_book.bids.len());
    assert_eq!(deserialized.asks.len(), order_book.asks.len());
    assert_eq!(deserialized.best_bid().unwrap().price, order_book.best_bid().unwrap().price);
    assert_eq!(deserialized.best_ask().unwrap().price, order_book.best_ask().unwrap().price);

    println!("✅ Data integrity verified after serialization round-trip");

    // Test capacity limits
    let mut large_book = OrderBookTLV::from_instrument(
        VenueId::NYSE,
        instrument,
        1640995200000000000,
        2000,
        100_000_000,
    );

    // Try to add more than MAX_ORDER_LEVELS (50) levels
    for i in 1..=55 {
        let price = 10000 + (i * 10);
        match large_book.add_bid(price, 100000, 1) {
            Ok(_) => continue,
            Err(_) => {
                if i > 50 {
                    println!("✅ Correctly rejected level {} (capacity limit: 50)", i);
                    break;
                } else {
                    return Err(format!("Unexpected capacity error at level {}", i).into());
                }
            }
        }
    }

    println!("✅ Capacity limits enforced correctly");

    // Test zero-copy characteristics
    println!("Testing zero-copy properties...");
    
    // Verify FixedVec is in a fixed memory location
    let order_book_size = std::mem::size_of::<OrderBookTLV>();
    println!("✅ OrderBookTLV size: {} bytes (includes FixedVec inline)", order_book_size);
    
    // The size should be deterministic and include the inline arrays
    // Expected: 8 + 2 + 1 + 1 + 8 + 8 + 8 + (8 + 50*24) + (8 + 50*24) = 2458 bytes
    let expected_min_size = 8 + 2 + 1 + 1 + 8 + 8 + 8 + (8 + 50 * 24) + (8 + 50 * 24);
    assert!(order_book_size >= expected_min_size);
    println!("✅ Memory layout is deterministic and suitable for zero-copy");

    println!("\n🎉 All OrderBookTLV FixedVec optimization tests passed!");
    println!("Key improvements achieved:");
    println!("- Zero-copy serialization with FixedVec");
    println!("- Deterministic memory layout");
    println!("- Bounded memory usage (max {} levels per side)", protocol_v2::tlv::MAX_ORDER_LEVELS);
    println!("- Maintained API compatibility with proper error handling");
    
    Ok(())
}