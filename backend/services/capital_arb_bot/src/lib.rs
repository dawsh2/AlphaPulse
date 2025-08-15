pub mod config;
pub mod dex;
pub mod executor;
pub mod simulator;

pub use config::Config;
pub use dex::{DexManager, DexRouter, TokenInfo};
pub use executor::CapitalArbExecutor;
pub use simulator::ArbSimulator;

#[cfg(test)]
mod tests;

use ethers::prelude::*;

#[derive(Debug, Clone)]
pub struct ArbOpportunity {
    pub timestamp_ns: u64,
    pub pair: String,
    pub token_a: Address,
    pub token_b: Address,
    pub dex_buy_router: Address,
    pub dex_sell_router: Address,
    pub price_a: f64,
    pub price_b: f64,
    pub liquidity_a: f64,
    pub liquidity_b: f64,
    pub estimated_profit_usd: f64,
    pub gas_estimate: u64,
}