// Quick test to verify OrderBookTLV macro conversion syntax
use libs::types::protocol::tlv::market_data::OrderBookTLV;
use libs::types::{InstrumentId, VenueId};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing OrderBookTLV macro conversion...");

    // Test macro-generated constructor
    let instrument = InstrumentId::stock(VenueId::NYSE, "AAPL");
    let order_book = OrderBookTLV::from_instrument(
        VenueId::NYSE,
        instrument,
        1640995200000000000, // timestamp_ns
        1000,                // sequence
        100_000_000,        // precision_factor for 8-decimal places
    );

    println!("✅ Created OrderBook with macro");
    println!("Size: {} bytes", std::mem::size_of::<OrderBookTLV>());
    println!("Bids: {}, Asks: {}", order_book.bids.len(), order_book.asks.len());

    // Test zero-copy serialization
    let bytes = order_book.as_bytes();
    println!("✅ Serialized to {} bytes", bytes.len());

    // Test zero-copy deserialization  
    let deserialized = OrderBookTLV::from_bytes(bytes)?;
    println!("✅ Deserialized successfully");

    // Verify field values with macro field order
    assert_eq!(order_book.timestamp_ns, deserialized.timestamp_ns);
    assert_eq!(order_book.asset_id, deserialized.asset_id);
    println!("✅ Field values match after roundtrip");

    println!("🎉 OrderBookTLV macro conversion working correctly!");
    Ok(())
}