//! Test fixtures and mock servers

pub mod mock_kraken;
pub mod test_data;

pub use mock_kraken::MockKrakenServer;
pub use test_data::*;

/// Mock arbitrage signal fixture for testing
pub struct ArbitrageSignalFixture {
    pub signal_id: u64,
    pub expected_profit_usd: f64,
    pub required_capital_usd: f64,
    pub timestamp_ns: u64,
}

impl ArbitrageSignalFixture {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
            
        Self {
            signal_id: now,
            expected_profit_usd: 42.50,
            required_capital_usd: 1000.0,
            timestamp_ns: now,
        }
    }
    
    pub fn with_profit(mut self, profit: f64) -> Self {
        self.expected_profit_usd = profit;
        self
    }
    
    pub fn with_capital(mut self, capital: f64) -> Self {
        self.required_capital_usd = capital;
        self
    }
}