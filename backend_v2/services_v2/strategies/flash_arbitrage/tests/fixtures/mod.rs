//! Test fixtures and data generators

use alphapulse_flash_arbitrage::{
    math::{V2PoolState, V3PoolState},
    pool_state::PoolState,
};
use alphapulse_protocol_v2::instrument_id::{PoolInstrumentId, VenueId};
use rand::Rng;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Generate realistic pool configurations
pub struct PoolFixtures;

impl PoolFixtures {
    /// Create a standard WETH/USDC V2 pool
    pub fn weth_usdc_v2() -> PoolState {
        PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![
                    0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2, // WETH
                    0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48, // USDC
                ],
                venue_id: VenueId::Uniswap as u16,
                pool_type: 2,
            },
            reserves: (dec!(1234.567890123456), dec!(2469135.780246)),
            fee_tier: 30,
            last_update_ns: 1700000000000000000,
        }
    }

    /// Create a WETH/USDC V3 pool
    pub fn weth_usdc_v3() -> PoolState {
        PoolState::V3 {
            pool_id: PoolInstrumentId {
                tokens: vec![
                    0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2, // WETH
                    0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48, // USDC
                ],
                venue_id: VenueId::UniswapV3 as u16,
                pool_type: 3,
            },
            liquidity: 123456789012345678,
            sqrt_price_x96: 1771845812700903892492943360, // ~2000 USDC per WETH
            current_tick: 201450,
            fee_tier: 500, // 0.05%
            last_update_ns: 1700000000000000000,
        }
    }

    /// Create pools with arbitrage opportunity
    pub fn arbitrage_pair() -> (PoolState, PoolState) {
        let pool_a = PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![1, 2],
                venue_id: VenueId::Uniswap as u16,
                pool_type: 2,
            },
            reserves: (dec!(1000), dec!(2000000)), // 1 ETH = 2000 USDC
            fee_tier: 30,
            last_update_ns: 1700000000000000000,
        };

        let pool_b = PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![1, 2],
                venue_id: VenueId::Sushiswap as u16,
                pool_type: 2,
            },
            reserves: (dec!(1050), dec!(1995000)), // 1 ETH = 1900 USDC (arbitrage!)
            fee_tier: 30,
            last_update_ns: 1700000000000000001,
        };

        (pool_a, pool_b)
    }

    /// Generate random V2 pool state
    pub fn random_v2_pool(rng: &mut impl Rng) -> V2PoolState {
        V2PoolState {
            reserve_in: Decimal::from(rng.gen_range(100..100000)),
            reserve_out: Decimal::from(rng.gen_range(100000..10000000)),
            fee_bps: *[25, 30, 100].iter().nth(rng.gen_range(0..3)).unwrap(),
        }
    }

    /// Generate random V3 pool state
    pub fn random_v3_pool(rng: &mut impl Rng) -> V3PoolState {
        V3PoolState {
            liquidity: rng.gen_range(1_000_000_000..1_000_000_000_000_000),
            sqrt_price_x96: rng.gen_range(4295128739..340282366920938463463374607431768211455u128),
            current_tick: rng.gen_range(-887272..887272),
            fee_pips: *[100, 500, 3000, 10000]
                .iter()
                .nth(rng.gen_range(0..4))
                .unwrap(),
        }
    }
}

/// Common token addresses on Polygon
pub struct PolygonTokens;

impl PolygonTokens {
    pub const WMATIC: u64 = 0x0d500B1d8E8eF31E;
    pub const USDC: u64 = 0x2791Bca1f2de4661;
    pub const USDT: u64 = 0xc2132D05D31c914a;
    pub const WETH: u64 = 0x7ceB23fD6bC0AdD5;
    pub const WBTC: u64 = 0x1bfd67037b42cf73;
    pub const DAI: u64 = 0x8f3Cf7ad23Cd3CaD;
}

/// Generate realistic arbitrage scenarios
pub struct ArbitrageScenarios;

impl ArbitrageScenarios {
    /// Stablecoin arbitrage (USDC/USDT)
    pub fn stablecoin_arb() -> (PoolState, PoolState) {
        let pool_a = PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![PolygonTokens::USDC, PolygonTokens::USDT],
                venue_id: VenueId::QuickSwap as u16,
                pool_type: 2,
            },
            reserves: (dec!(1000000), dec!(999500)), // Slight depeg
            fee_tier: 5,                             // 0.05% for stables
            last_update_ns: 1700000000000000000,
        };

        let pool_b = PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![PolygonTokens::USDC, PolygonTokens::USDT],
                venue_id: VenueId::Sushiswap as u16,
                pool_type: 2,
            },
            reserves: (dec!(2000000), dec!(2002000)), // Different ratio
            fee_tier: 5,
            last_update_ns: 1700000000000000001,
        };

        (pool_a, pool_b)
    }

    /// High volatility scenario (WBTC/WETH)
    pub fn volatile_arb() -> (PoolState, PoolState) {
        let pool_a = PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![PolygonTokens::WBTC, PolygonTokens::WETH],
                venue_id: VenueId::Uniswap as u16,
                pool_type: 2,
            },
            reserves: (dec!(100), dec!(1500)), // 1 BTC = 15 ETH
            fee_tier: 30,
            last_update_ns: 1700000000000000000,
        };

        let pool_b = PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![PolygonTokens::WBTC, PolygonTokens::WETH],
                venue_id: VenueId::QuickSwap as u16,
                pool_type: 2,
            },
            reserves: (dec!(80), dec!(1280)), // 1 BTC = 16 ETH (arbitrage!)
            fee_tier: 30,
            last_update_ns: 1700000000000000001,
        };

        (pool_a, pool_b)
    }
}
