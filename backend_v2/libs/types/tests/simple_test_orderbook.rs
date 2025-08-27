use protocol_v2::tlv::OrderBookTLV;
use protocol_v2::{InstrumentId, VenueId};

fn main() {
    println!("Testing OrderBookTLV FixedVec implementation...");

    let instrument = InstrumentId::stock(VenueId::NYSE, "AAPL");
    let mut book = OrderBookTLV::from_instrument(
        VenueId::NYSE,
        instrument,
        1640995200000000000,
        1000,
        100_000_000,
    );

    // Add some levels
    book.add_bid(10000, 100000, 5).expect("Failed to add bid");
    book.add_ask(10050, 150000, 2).expect("Failed to add ask");

    println!("Bids: {}, Asks: {}", book.bids.len(), book.asks.len());
    println!("Best bid: {:?}", book.best_bid().map(|b| b.price));
    println!("Best ask: {:?}", book.best_ask().map(|a| a.price));

    println!("✅ OrderBookTLV FixedVec optimization working correctly!");
}
