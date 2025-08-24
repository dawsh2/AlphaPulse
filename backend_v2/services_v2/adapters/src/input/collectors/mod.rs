//! Venue-specific data collectors

pub mod binance;
pub mod coinbase;
// pub mod gemini;  // Disabled due to compilation errors
pub mod kraken;
// pub mod polygon;  // Replaced by unified bin/polygon/polygon.rs (direct relay, no MPSC)

#[cfg(test)]
mod tests;

pub use binance::BinanceCollector;
pub use coinbase::CoinbaseCollector;
// pub use gemini::GeminiCollector;  // Disabled due to compilation errors
pub use kraken::KrakenCollector;
// pub use polygon::PolygonDexCollector;  // Replaced by unified bin/polygon/polygon.rs
