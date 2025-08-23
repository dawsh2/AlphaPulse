pub mod config;
pub mod opportunity_detector;
pub mod pool_monitor;
// pub mod pool_fetcher; // TODO: Fix compilation errors
pub mod price_calculator;
pub mod exchanges;
pub mod pool_state;
pub mod amm_math;
pub mod execution_interface;
pub mod gas_estimation;
pub mod v3_math;
// pub mod test_opportunities;
// pub mod execution_test;
pub mod huff_gas_estimator;
// pub mod streaming_pipeline;
pub mod mumbai_config;
pub mod amoy_config;
pub mod live_dashboard;
pub mod text_dashboard;
// pub mod live_monitor;

pub use opportunity_detector::OpportunityDetector;
pub use pool_monitor::PoolMonitor;
// pub use pool_fetcher::PoolFetcher; // TODO: Fix compilation
pub use price_calculator::PriceCalculator;
pub use huff_gas_estimator::HuffGasEstimator;
pub use pool_state::{PoolState, TokenMetadata, PoolStateStats};
pub use amm_math::{AmmMath, ArbitrageProfitability};
pub use execution_interface::{ExecutionInterface, ChannelExecutionInterface, MockExecutionInterface, ExecutionStatus, QueueStats, ExecutionConfig};
pub use gas_estimation::{GasCalculator, ContractType, ArbitrageCharacteristics};
pub use mumbai_config::{MumbaiConfig, MumbaiContracts, MumbaiOptimizations};
pub use amoy_config::{AmoyConfig, AmoyContracts, AmoyOptimizations, AmoyAmmIntegration};
pub use live_dashboard::{LiveDashboard, FlashType};
pub use text_dashboard::TextDashboard;

use zerocopy::AsBytes;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

/// Represents an arbitrage opportunity detected across DEXs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub id: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: Decimal,
    pub amount_out: Decimal,
    pub profit_usd: Decimal,
    pub profit_percentage: Decimal,
    pub buy_exchange: String,
    pub sell_exchange: String,
    pub buy_pool: String,
    pub sell_pool: String,
    pub gas_cost_estimate: Decimal,
    pub net_profit_usd: Decimal,
    pub timestamp: i64,
    pub block_number: u64,
    pub confidence_score: f64,
}

/// Simple arbitrage opportunity for dashboard display
#[derive(Debug, Clone)]
pub struct SimpleArbitrageOpportunity {
    pub buy_pool: PoolInfo,
    pub sell_pool: PoolInfo,
    pub buy_amount_in: Decimal,
    pub expected_profit_usd: Decimal,
    pub gas_cost_usd: Decimal,
    pub confidence_score: f64,
}

/// Events published by OpportunityDetector for dashboard consumption
#[derive(Debug, Clone)]
pub enum DashboardUpdate {
    /// New arbitrage opportunity detected
    NewOpportunity(ArbitrageOpportunity),
    /// Pool group information updated (same token pair across multiple DEXs)
    PoolGroupUpdate {
        token_pair: String,
        token0_symbol: String,
        token1_symbol: String,
        pools: Vec<PoolInfo>,
        price_range: (Decimal, Decimal), // min, max price
        max_spread_percent: Decimal,
        total_liquidity_usd: Option<Decimal>,
        best_opportunity: Option<ArbitrageOpportunity>,
    },
    /// Token symbol successfully resolved
    TokenSymbolResolved {
        address: String,
        symbol: String,
    },
}

impl ArbitrageOpportunity {
    /// Convert to enhanced binary ArbitrageOpportunityMessage for relay broadcast
    pub fn to_binary_message(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use alphapulse_protocol::messages::ArbitrageOpportunityMessage;
        use alphapulse_protocol::message_protocol::SourceType;
        use zerocopy::AsBytes;
        
        // Convert decimal values to fixed-point integers (8 decimal precision)
        let trade_size_usd = (self.amount_in * Decimal::new(100000000, 0))
            .to_string().parse::<u64>().unwrap_or(0);
        let gross_profit_usd = (self.profit_usd * Decimal::new(100000000, 0))
            .to_string().parse::<u64>().unwrap_or(0);
        let gas_fee_usd = (self.gas_cost_estimate * Decimal::new(100000000, 0))
            .to_string().parse::<u64>().unwrap_or(0);
        let net_profit_usd = (self.net_profit_usd * Decimal::new(100000000, 0))
            .to_string().parse::<u64>().unwrap_or(0);
        
        // Convert profit percentage to fixed-point (4 decimal precision for percentages)
        let profit_percent = (self.profit_percentage * Decimal::new(10000, 0))
            .to_string().parse::<u32>().unwrap_or(0);
        
        // Get token symbols from addresses (truncated)
        let token0_symbol = self.get_token_symbol(&self.token_in);
        let token1_symbol = self.get_token_symbol(&self.token_out);
        
        // Generate InstrumentIds
        let token0_id = self.generate_token_instrument_id(&self.token_in);
        let token1_id = self.generate_token_instrument_id(&self.token_out);
        let buy_pool_id = self.generate_pool_instrument_id(&self.buy_pool);
        let sell_pool_id = self.generate_pool_instrument_id(&self.sell_pool);
        
        let arb_msg = ArbitrageOpportunityMessage::new(
            token0_id,
            token1_id,
            buy_pool_id,
            sell_pool_id,
            0, // buy_price (calculated from reserves)
            0, // sell_price (calculated from reserves)
            trade_size_usd,
            gross_profit_usd,
            gas_fee_usd,
            0, // dex_fees_usd (TODO: calculate from exchange fees)
            0, // slippage_cost_usd (TODO: calculate slippage)
            net_profit_usd,
            profit_percent,
            (self.confidence_score * 1000.0) as u16, // Convert to 3 decimal precision
            true, // executable
            &token0_symbol,
            &token1_symbol,
            &self.buy_exchange,
            &self.sell_exchange,
            self.timestamp as u64,
            SourceType::Scanner,
        );

        Ok(arb_msg.as_bytes().to_vec())
    }
    
    /// Get simplified token symbol from address
    fn get_token_symbol(&self, address: &str) -> String {
        // For now, use a simple truncated address as symbol
        // In production, this could resolve from token registry
        if address.len() >= 10 {
            format!("{}...{}", &address[0..6], &address[address.len()-4..])
        } else {
            address.to_string()
        }
    }
    
    /// Generate InstrumentId for a token address
    fn generate_token_instrument_id(&self, address: &str) -> alphapulse_protocol::InstrumentId {
        alphapulse_protocol::InstrumentId::polygon_token(address)
            .unwrap_or_else(|_| {
                // Fallback to a hash-based ID if address parsing fails
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                address.hash(&mut hasher);
                let hash = hasher.finish();
                
                alphapulse_protocol::InstrumentId {
                    venue: alphapulse_protocol::VenueId::Polygon as u16,
                    asset_type: alphapulse_protocol::AssetType::Token as u8,
                    reserved: 0,
                    asset_id: hash,
                }
            })
    }
    
    /// Generate InstrumentId for a pool address
    fn generate_pool_instrument_id(&self, address: &str) -> alphapulse_protocol::InstrumentId {
        // For pools, use a similar approach with pool asset type
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        address.hash(&mut hasher);
        let hash = hasher.finish();
        
        alphapulse_protocol::InstrumentId {
            venue: alphapulse_protocol::VenueId::Polygon as u16,
            asset_type: alphapulse_protocol::AssetType::Pool as u8,
            reserved: 0,
            asset_id: hash,
        }
    }

    /// Generate InstrumentId for this arbitrage opportunity using bijective ID system
    fn generate_instrument_id(&self) -> alphapulse_protocol::InstrumentId {
        // Create bijective ID from token symbols - use polygon_token as default
        // TODO: Improve this to be more specific to the trading pair
        let token0_id = alphapulse_protocol::InstrumentId::polygon_token(&self.token_in)
            .unwrap_or_else(|_| alphapulse_protocol::InstrumentId::from_u64(0));
        let token1_id = alphapulse_protocol::InstrumentId::polygon_token(&self.token_out)
            .unwrap_or_else(|_| alphapulse_protocol::InstrumentId::from_u64(0));
            
        // Create pool ID from tokens (represents the arbitrage opportunity)
        alphapulse_protocol::InstrumentId::pool(
            alphapulse_protocol::VenueId::Polygon, 
            token0_id, 
            token1_id
        )
    }
}

/// Exchange-agnostic pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: String,
    pub exchange: String,
    pub token0: String,
    pub token1: String,
    pub reserve0: Decimal,
    pub reserve1: Decimal,
    pub fee: Decimal,
    pub last_updated: i64,
    pub block_number: u64,
    
    // V3-specific fields for tick-based liquidity
    pub v3_tick: Option<i32>,
    pub v3_sqrt_price_x96: Option<u128>,
    pub v3_liquidity: Option<u128>,
}

/// Price quote from a specific exchange
#[derive(Debug, Clone)]
pub struct PriceQuote {
    pub exchange: String,
    pub pool: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: Decimal,
    pub amount_out: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
    pub slippage: Decimal,
    pub timestamp: i64,
}

/// Configuration for minimum profitable arbitrage
#[derive(Debug, Clone)]
pub struct ArbitrageConfig {
    pub min_profit_usd: Decimal,
    pub min_profit_percentage: Decimal,
    pub max_gas_cost_usd: Decimal,
    pub min_liquidity_usd: Decimal,
    pub max_slippage_percentage: Decimal,
    pub confidence_threshold: f64,
}