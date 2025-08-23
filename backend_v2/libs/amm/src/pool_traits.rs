//! Pool trait definitions for unified AMM interface

use crate::{Decimal, V2Math, V2PoolState};
use anyhow::Result;

/// Pool type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolType {
    UniswapV2,
    UniswapV3,
    SushiSwap,
    QuickSwap,
}

/// Unified pool interface for arbitrage calculations
pub trait AmmPool {
    /// Calculate output amount for given input
    fn get_amount_out(&self, amount_in: Decimal) -> Result<Decimal>;

    /// Calculate required input for desired output
    fn get_amount_in(&self, amount_out: Decimal) -> Result<Decimal>;

    /// Get current reserves or liquidity
    fn get_liquidity(&self) -> (Decimal, Decimal);

    /// Get fee tier
    fn get_fee_bps(&self) -> u32;
}

impl AmmPool for V2PoolState {
    fn get_amount_out(&self, amount_in: Decimal) -> Result<Decimal> {
        V2Math::calculate_output_amount(amount_in, self.reserve_in, self.reserve_out, self.fee_bps)
    }

    fn get_amount_in(&self, amount_out: Decimal) -> Result<Decimal> {
        V2Math::calculate_input_amount(amount_out, self.reserve_in, self.reserve_out, self.fee_bps)
    }

    fn get_liquidity(&self) -> (Decimal, Decimal) {
        (self.reserve_in, self.reserve_out)
    }

    fn get_fee_bps(&self) -> u32 {
        self.fee_bps
    }
}
