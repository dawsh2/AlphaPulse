//! Mock implementations for testing

use alphapulse_flash_arbitrage::pool_state::PoolState;
use alphapulse_protocol_v2::{
    instrument_id::{InstrumentId, PoolInstrumentId, VenueId},
    tlv::{TLVMessageBuilder, TLVType},
    MessageHeader, SourceType,
};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Mock DEX data feed
pub struct MockDexFeed {
    tx: mpsc::Sender<Vec<u8>>,
    running: Arc<RwLock<bool>>,
}

impl MockDexFeed {
    pub fn new(tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start generating mock swap events
    pub async fn start(&self) {
        *self.running.write() = true;
        let tx = self.tx.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut sequence = 0u32;

            while *running.read() {
                // Generate mock trade TLV
                let mut builder = TLVMessageBuilder::new(SourceType::PolygonCollector, sequence);

                // Create pool ID
                let pool_id = PoolInstrumentId {
                    tokens: vec![1, 2],
                    venue_id: VenueId::Uniswap as u16,
                    pool_type: 2,
                };

                // Add trade data (simplified)
                let trade_data = vec![0u8; 48]; // Mock trade data
                builder.add_tlv(TLVType::Trade, &trade_data).unwrap();

                let message = builder.build().unwrap();
                let _ = tx.send(message).await;

                sequence += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
    }

    pub fn stop(&self) {
        *self.running.write() = false;
    }
}

/// Mock message bus for testing
pub struct MockMessageBus {
    channels: Arc<RwLock<Vec<mpsc::Sender<Vec<u8>>>>>,
}

impl MockMessageBus {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn subscribe(&self) -> mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = mpsc::channel(100);
        self.channels.write().push(tx);
        rx
    }

    pub async fn publish(&self, message: Vec<u8>) {
        let channels = self.channels.read().clone();
        for tx in channels {
            let _ = tx.send(message.clone()).await;
        }
    }
}

/// Mock pool generator
pub struct MockPoolGenerator;

impl MockPoolGenerator {
    /// Generate a pool with specific price ratio
    pub fn create_pool(tokens: Vec<u64>, venue: VenueId, price_ratio: Decimal) -> PoolState {
        PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens,
                venue_id: venue as u16,
                pool_type: 2,
            },
            reserves: (dec!(1000), dec!(1000) * price_ratio),
            fee_tier: 30,
            last_update_ns: 1000000,
        }
    }

    /// Generate pools with arbitrage opportunity
    pub fn create_arbitrage_pools() -> (PoolState, PoolState) {
        let pool_a = Self::create_pool(
            vec![1, 2],
            VenueId::Uniswap,
            dec!(2000), // 1:2000 ratio
        );

        let pool_b = Self::create_pool(
            vec![1, 2],
            VenueId::Sushiswap,
            dec!(1900), // 1:1900 ratio (arbitrage!)
        );

        (pool_a, pool_b)
    }

    /// Generate V3 pool
    pub fn create_v3_pool(tokens: Vec<u64>, venue: VenueId, sqrt_price: u128) -> PoolState {
        PoolState::V3 {
            pool_id: PoolInstrumentId {
                tokens,
                venue_id: venue as u16,
                pool_type: 3,
            },
            liquidity: 1_000_000_000_000,
            sqrt_price_x96: sqrt_price,
            current_tick: 0,
            fee_tier: 500,
            last_update_ns: 1000000,
        }
    }
}

/// Mock blockchain client for testing execution
pub struct MockBlockchainClient {
    success_rate: f64,
    gas_price: u64,
}

impl MockBlockchainClient {
    pub fn new(success_rate: f64) -> Self {
        Self {
            success_rate,
            gas_price: 50, // 50 gwei
        }
    }

    /// Simulate transaction submission
    pub async fn submit_transaction(&self, _data: Vec<u8>) -> Result<String, String> {
        // Simulate network delay
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Simulate success/failure based on rate
        if rand::random::<f64>() < self.success_rate {
            Ok(format!("0x{:064x}", rand::random::<u64>()))
        } else {
            Err("Transaction failed".to_string())
        }
    }

    pub fn estimate_gas(&self) -> u64 {
        200_000 // Typical flash loan gas usage
    }

    pub fn get_gas_price(&self) -> u64 {
        self.gas_price
    }
}

/// Mock price oracle
pub struct MockPriceOracle {
    prices: Arc<RwLock<std::collections::HashMap<u64, Decimal>>>,
}

impl MockPriceOracle {
    pub fn new() -> Self {
        let mut prices = std::collections::HashMap::new();

        // Default prices
        prices.insert(1, dec!(2000)); // ETH
        prices.insert(2, dec!(1)); // USDC
        prices.insert(3, dec!(1)); // USDT
        prices.insert(4, dec!(40000)); // BTC

        Self {
            prices: Arc::new(RwLock::new(prices)),
        }
    }

    pub fn get_price(&self, token_id: u64) -> Option<Decimal> {
        self.prices.read().get(&token_id).copied()
    }

    pub fn set_price(&self, token_id: u64, price: Decimal) {
        self.prices.write().insert(token_id, price);
    }

    /// Simulate price volatility
    pub fn add_volatility(&self, token_id: u64, percent: Decimal) {
        if let Some(price) = self.get_price(token_id) {
            let variation = price * percent / dec!(100);
            let new_price = if rand::random::<bool>() {
                price + variation
            } else {
                price - variation
            };
            self.set_price(token_id, new_price);
        }
    }
}
