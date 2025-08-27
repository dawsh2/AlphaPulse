//! Polygon DEX adapter implementation
//!
//! Handles Polygon/Matic network DEX events and pool state management.
//! Processes Uniswap V2, V3, and other AMM events from Polygon chain.

pub mod collector;
pub mod parser;
pub mod types;

use alphapulse_types::protocol::{MessageHeader, TLVType};
use alphapulse_codec::TlvTypeRegistry;
use anyhow::Result;

/// Main Polygon adapter for processing DEX events
pub struct PolygonAdapter {
    collector: collector::PolygonCollector,
    parser: parser::PolygonEventParser,
}

impl PolygonAdapter {
    /// Create new Polygon adapter instance
    pub fn new(config: PolygonConfig) -> Result<Self> {
        let collector = collector::PolygonCollector::new(config.rpc_url)?;
        let parser = parser::PolygonEventParser::new();
        
        Ok(Self { collector, parser })
    }
    
    /// Process incoming Polygon events
    pub async fn process_events(&mut self) -> Result<Vec<MessageHeader>> {
        let events = self.collector.fetch_events().await?;
        let messages = self.parser.parse_events(events)?;
        Ok(messages)
    }
}

/// Configuration for Polygon adapter
#[derive(Debug, Clone)]
pub struct PolygonConfig {
    pub rpc_url: String,
    pub chain_id: u64,
    pub block_confirmations: u64,
    pub max_events_per_batch: usize,
}