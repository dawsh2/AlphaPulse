//! TLV to JSON message conversion for dashboard

use crate::error::{DashboardError, Result};
use base64::prelude::*;
use protocol_v2::InstrumentId;
use protocol_v2::{
    tlv::{ArbitrageSignalTLV, DemoDeFiArbitrageTLV, PoolSyncTLV},
    ParseError, PoolSwapTLV, QuoteTLV, VenueId,
};
use serde_json::{json, Value};
use std::time::SystemTime;

/// Convert TLV message to JSON for dashboard consumption
pub fn convert_tlv_to_json(tlv_type: u8, payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    match tlv_type {
        1 => convert_trade_tlv(payload, timestamp_ns), // TLVType::Trade
        2 => convert_quote_tlv(payload, timestamp_ns), // TLVType::Quote
        3 => convert_state_invalidation_tlv(payload, timestamp_ns), // TLVType::StateInvalidation
        10 => convert_pool_liquidity_tlv(payload, timestamp_ns), // PoolLiquidityTLV
        11 => convert_pool_swap_tlv(payload, timestamp_ns), // PoolSwapTLV
        12 => convert_pool_mint_tlv(payload, timestamp_ns), // PoolMintTLV
        13 => convert_pool_burn_tlv(payload, timestamp_ns), // PoolBurnTLV
        14 => convert_pool_tick_tlv(payload, timestamp_ns), // PoolTickTLV
        16 => convert_pool_sync_tlv(payload, timestamp_ns), // PoolSyncTLV
        32 => convert_arbitrage_signal_tlv(payload, timestamp_ns), // ArbitrageSignalTLV
        67 => convert_flash_loan_result_tlv(payload, timestamp_ns), // TLVType::FlashLoanResult
        202 => convert_proprietary_data_tlv(payload, timestamp_ns), // VendorTLVType::ProprietaryData
        255 => convert_demo_defi_arbitrage_tlv(payload, timestamp_ns), // DemoDeFiArbitrageTLV
        _ => Ok(json!({
            "msg_type": "unknown",
            "tlv_type": tlv_type,
            "timestamp": timestamp_ns,
            "raw_data": base64::prelude::BASE64_STANDARD.encode(payload)
        })),
    }
}

fn convert_trade_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    if payload.len() < 22 {
        return Err(DashboardError::Protocol(protocol_v2::ProtocolError::Parse(
            ParseError::MessageTooSmall { need: 22, got: 0 },
        )));
    }

    // Parse instrument ID
    let venue = u16::from_le_bytes([payload[0], payload[1]]);
    let asset_type = payload[2];
    let reserved = payload[3];
    let asset_id = u64::from_le_bytes([
        payload[4],
        payload[5],
        payload[6],
        payload[7],
        payload[8],
        payload[9],
        payload[10],
        payload[11],
    ]);

    let instrument_id = InstrumentId {
        venue,
        asset_type,
        reserved,
        asset_id,
    };

    // Parse price and volume
    let price_raw = i64::from_le_bytes([
        payload[12],
        payload[13],
        payload[14],
        payload[15],
        payload[16],
        payload[17],
        payload[18],
        payload[19],
    ]);
    let volume_raw = u64::from_le_bytes([payload[20], payload[21], 0, 0, 0, 0, 0, 0]);
    let side = payload[22];

    // Convert to human-readable format
    let price = price_raw as f64 / 100_000_000.0; // Fixed-point to decimal
    let volume = volume_raw as f64 / 100_000_000.0;

    Ok(json!({
        "msg_type": "trade",
        "instrument": {
            "venue": venue,
            "venue_name": format!("Venue{}", venue),
            "symbol": instrument_id.debug_info(),
            "asset_type": asset_type
        },
        "price": price,
        "volume": volume,
        "side": match side {
            1 => "buy",
            2 => "sell",
            _ => "unknown"
        },
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns)
    }))
}

fn convert_quote_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    let quote = QuoteTLV::from_bytes(payload).map_err(|_e| {
        DashboardError::Protocol(protocol_v2::ProtocolError::Parse(
            ParseError::MessageTooSmall {
                need: 32,
                got: payload.len(),
            },
        ))
    })?;

    // Copy packed fields to local variables to avoid unaligned references
    let bid_price = quote.bid_price;
    let ask_price = quote.ask_price;
    let bid_size = quote.bid_size;
    let ask_size = quote.ask_size;

    Ok(json!({
        "msg_type": "quote",
        "instrument_id": quote.instrument_id().to_u64(),
        "bid_price": bid_price,
        "ask_price": ask_price,
        "bid_size": bid_size,
        "ask_size": ask_size,
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns)
    }))
}

fn convert_state_invalidation_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    // Simple parsing for StateInvalidationTLV - extract basic fields
    if payload.len() < 12 {
        // minimum: venue(2) + sequence(8) + count(2)
        return Err(DashboardError::Protocol(protocol_v2::ProtocolError::Parse(
            ParseError::MessageTooSmall {
                need: 12,
                got: payload.len(),
            },
        )));
    }

    let venue_id = u16::from_le_bytes([payload[0], payload[1]]);
    let sequence = u64::from_le_bytes([
        payload[2], payload[3], payload[4], payload[5], payload[6], payload[7], payload[8],
        payload[9],
    ]);
    let instrument_count = u16::from_le_bytes([payload[10], payload[11]]);

    // For dashboard purposes, create a simple representation
    let invalidation_data = json!({
        "venue_id": venue_id,
        "sequence": sequence,
        "instrument_count": instrument_count,
        "reason": "StateInvalidation"
    });

    Ok(json!({
        "msg_type": "state_invalidation",
        "data": invalidation_data,
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns)
    }))
}

fn get_strategy_name(strategy_id: u16) -> &'static str {
    match strategy_id {
        20 => "Kraken Signals",
        21 => "Flash Arbitrage",
        22 => "Cross-Chain Arbitrage",
        _ => "Unknown Strategy",
    }
}

fn timestamp_to_iso(timestamp_ns: u64) -> String {
    let timestamp_secs = timestamp_ns / 1_000_000_000;
    let datetime =
        match SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(timestamp_secs)) {
            Some(dt) => dt,
            None => SystemTime::now(),
        };

    // Convert to ISO string (simplified)
    format!("{:?}", datetime)
}

/// Create a combined signal message from multiple TLVs
pub fn create_combined_signal(
    signal_identity: Option<Value>,
    economics: Option<Value>,
    timestamp_ns: u64,
) -> Value {
    let mut combined = json!({
        "msg_type": "trading_signal",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns)
    });

    if let Some(identity) = signal_identity {
        combined["signal_id"] = identity["signal_id"].clone();
        combined["strategy_id"] = identity["strategy_id"].clone();
        combined["strategy_name"] = identity["strategy_name"].clone();
        combined["confidence"] = identity["confidence"].clone();
    }

    if let Some(econ) = economics {
        combined["expected_profit_usd"] = econ["expected_profit_usd"].clone();
        combined["required_capital_usd"] = econ["required_capital_usd"].clone();
        combined["profit_margin_pct"] = econ["profit_margin_pct"].clone();
    }

    combined
}

/// Create arbitrage opportunity message for dashboard
pub fn create_arbitrage_opportunity(
    signal_identity: Option<Value>,
    economics: Option<Value>,
    timestamp_ns: u64,
) -> Value {
    let mut opportunity = json!({
        "msg_type": "arbitrage_opportunity",
        "detected_at": timestamp_ns,
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns)
    });

    // Add signal identity data
    if let Some(identity) = signal_identity {
        opportunity["signal_id"] = identity["signal_id"].clone();
        opportunity["strategy_id"] = identity["strategy_id"].clone();
        opportunity["strategy_name"] = identity["strategy_name"].clone();
        opportunity["confidence_score"] = identity["confidence"].clone();
    }

    // Add economics data in dashboard-expected format
    if let Some(econ) = economics {
        opportunity["estimated_profit"] = econ["expected_profit_usd"].clone();
        opportunity["net_profit_usd"] = econ["expected_profit_usd"].clone();
        opportunity["max_trade_size"] = econ["required_capital_usd"].clone();
        opportunity["profit_percent"] = econ["profit_margin_pct"].clone();
        opportunity["net_profit_percent"] = econ["profit_margin_pct"].clone();
        opportunity["executable"] = serde_json::Value::Bool(true);

        // Default values for fields the dashboard expects
        opportunity["pair"] = json!("UNKNOWN-PAIR");
        opportunity["token_a"] = json!("0x0000000000000000000000000000000000000000");
        opportunity["token_b"] = json!("0x0000000000000000000000000000000000000000");
        opportunity["dex_buy"] = json!("QuickSwap");
        opportunity["dex_sell"] = json!("SushiSwap");
        opportunity["dex_buy_router"] = json!("0x0000000000000000000000000000000000000000");
        opportunity["dex_sell_router"] = json!("0x0000000000000000000000000000000000000000");
        opportunity["price_buy"] = json!(0.0);
        opportunity["price_sell"] = json!(0.0);
        opportunity["gas_fee_usd"] = json!(2.5);
        opportunity["dex_fees_usd"] = json!(3.0);
        opportunity["slippage_cost_usd"] = json!(1.0);
    }

    opportunity
}

/// Convert ArbitrageSignalTLV to arbitrage opportunity JSON
fn convert_arbitrage_signal_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    let signal = ArbitrageSignalTLV::from_bytes(payload).map_err(|_| {
        DashboardError::Protocol(protocol_v2::ProtocolError::Parse(
            ParseError::MessageTooSmall {
                need: 168,
                got: payload.len(),
            },
        ))
    })?;

    // Copy packed fields to avoid unaligned references
    let source_venue = signal.source_venue;
    let target_venue = signal.target_venue;
    let token_in = signal.token_in;
    let token_out = signal.token_out;
    let source_pool = signal.source_pool;
    let target_pool = signal.target_pool;
    let strategy_id = signal.strategy_id;
    let signal_id = signal.signal_id;
    let chain_id = signal.chain_id;
    let slippage_tolerance_bps = signal.slippage_tolerance_bps;
    let max_gas_price_gwei = signal.max_gas_price_gwei;
    let valid_until = signal.valid_until;
    let priority = signal.priority;

    // Map venue IDs to DEX names - improved mapping for Polygon DEXs
    let dex_buy = match source_venue {
        x if x == VenueId::UniswapV2 as u16 => "Uniswap V2",
        x if x == VenueId::UniswapV3 as u16 => "Uniswap V3",
        x if x == VenueId::SushiSwap as u16 => "SushiSwap",
        x if x == VenueId::SushiSwapPolygon as u16 => "SushiSwap",
        x if x == VenueId::QuickSwap as u16 => "QuickSwap",
        x if x == VenueId::CurvePolygon as u16 => "Curve",
        x if x == VenueId::BalancerPolygon as u16 => "Balancer",
        x if x == VenueId::Polygon as u16 => "QuickSwap", // Fallback: treat blockchain ID as DEX
        202 => "QuickSwap",                               // Direct numeric fallback for venue 202
        _ => "Unknown DEX",
    };

    let dex_sell = match target_venue {
        x if x == VenueId::UniswapV2 as u16 => "Uniswap V2",
        x if x == VenueId::UniswapV3 as u16 => "Uniswap V3",
        x if x == VenueId::SushiSwap as u16 => "SushiSwap",
        x if x == VenueId::SushiSwapPolygon as u16 => "SushiSwap",
        x if x == VenueId::QuickSwap as u16 => "QuickSwap",
        x if x == VenueId::CurvePolygon as u16 => "Curve",
        x if x == VenueId::BalancerPolygon as u16 => "Balancer",
        x if x == VenueId::Polygon as u16 => "SushiSwap", // Fallback: alternate DEX for differentiation
        202 => "SushiSwap",                               // Direct numeric fallback for venue 202
        _ => "Unknown DEX",
    };

    Ok(json!({
        "msg_type": "arbitrage_opportunity",
        "type": "real_arbitrage",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),

        // Pool and token information
        "pair": format!("{}/{}",
            format_token_address(&token_in),
            format_token_address(&token_out)
        ),
        "token_a": hex::encode(token_in),
        "token_b": hex::encode(token_out),
        "pool_a": hex::encode(source_pool),
        "pool_b": hex::encode(target_pool),
        "dex_buy": dex_buy,
        "dex_sell": dex_sell,
        "buyExchange": dex_buy, // Alternative field name
        "sellExchange": dex_sell, // Alternative field name

        // Financial metrics
        "estimated_profit": signal.expected_profit_usd(),
        "net_profit_usd": signal.net_profit_usd(),
        "max_trade_size": signal.required_capital_usd(),
        "required_capital_usd": signal.required_capital_usd(),
        "spread": signal.spread_percent(),
        "spread_percent": signal.spread_percent(),
        "profit_percent": (signal.net_profit_usd() / signal.required_capital_usd() * 100.0),
        "net_profit_percent": (signal.net_profit_usd() / signal.required_capital_usd() * 100.0),

        // Cost breakdown
        "gas_fee_usd": signal.gas_cost_usd(),
        "dex_fees_usd": signal.dex_fees_usd(),
        "slippage_cost_usd": signal.slippage_usd(),

        // Trading parameters
        "slippage_tolerance": slippage_tolerance_bps as f64 / 100.0,
        "max_gas_price_gwei": max_gas_price_gwei,
        "valid_until": valid_until,
        "priority": priority,
        "executable": signal.is_valid((timestamp_ns / 1_000_000_000) as u32),

        // Strategy metadata
        "strategy_id": strategy_id,
        "signal_id": signal_id.to_string(),
        "chain_id": chain_id,
    }))
}

/// Helper to format token address for display
fn format_token_address(addr: &[u8; 20]) -> String {
    let hex_str = hex::encode(addr);
    // Show first 6 and last 4 chars
    if hex_str.len() >= 10 {
        format!("0x{}...{}", &hex_str[..6], &hex_str[hex_str.len() - 4..])
    } else {
        format!("0x{}", hex_str)
    }
}

/// Map truncated 64-bit token IDs to symbols for Polygon tokens
fn map_token_symbol(token_id: u64) -> &'static str {
    match token_id {
        0x2791bca1f2de4661u64 => "USDC", // USDC on Polygon: 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174
        0x0d500b1d8e8ef31eu64 => "WMATIC", // WMATIC on Polygon: 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270
        0x7ceB23fD6bC0adDBu64 => "WETH", // WETH on Polygon: 0x7ceB23fD6bC0adDBd44Bd6f21b62d628Fc157ae1
        0xc2132d05d31c914au64 => "USDT", // USDT on Polygon: 0xc2132D05D31c914a87C6611C10748AEb04B58e8F
        0x8f3cf7ad23cd3cabu64 => "DAI",  // DAI on Polygon: 0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063 (truncated)
        0x1bfd67037b42cf73u64 => "WBTC", // WBTC on Polygon: 0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6 (truncated)
        _ => "UNKNOWN", // Fallback for unknown tokens
    }
}

/// Convert DemoDeFiArbitrageTLV to arbitrage opportunity JSON with enhanced metrics
fn convert_demo_defi_arbitrage_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    use zerocopy::FromBytes;
    let arbitrage_tlv = DemoDeFiArbitrageTLV::ref_from(payload).ok_or_else(|| {
        DashboardError::Protocol(protocol_v2::ProtocolError::Parse(
            ParseError::MessageTooSmall {
                need: std::mem::size_of::<DemoDeFiArbitrageTLV>(),
                got: payload.len(),
            },
        ))
    })?;

    // Copy packed fields to local variables to avoid unaligned reference errors
    let strategy_id = arbitrage_tlv.strategy_id;
    let signal_id = arbitrage_tlv.signal_id;
    let confidence = arbitrage_tlv.confidence;
    let chain_id = arbitrage_tlv.chain_id;
    let expected_profit_q = arbitrage_tlv.expected_profit_q;
    let required_capital_q = arbitrage_tlv.required_capital_q;
    let estimated_gas_cost_q = arbitrage_tlv.estimated_gas_cost_q;
    let venue_a = arbitrage_tlv.venue_a;
    let venue_b = arbitrage_tlv.venue_b;
    let pool_a = arbitrage_tlv.pool_a;
    let pool_b = arbitrage_tlv.pool_b;
    let token_in = arbitrage_tlv.token_in;
    let token_out = arbitrage_tlv.token_out;
    let optimal_amount_q = arbitrage_tlv.optimal_amount_q;
    let slippage_tolerance = arbitrage_tlv.slippage_tolerance;
    let max_gas_price_gwei = arbitrage_tlv.max_gas_price_gwei;
    let valid_until = arbitrage_tlv.valid_until;
    let priority = arbitrage_tlv.priority;
    let timestamp_ns = arbitrage_tlv.timestamp_ns;

    // Extract pool information using copied values
    let pool_a_venues = match venue_a {
        300 => "Uniswap V2",           // UniswapV2
        301 => "Uniswap V3",           // UniswapV3
        302 => "SushiSwap",            // SushiSwap (Ethereum)
        400 => "QuickSwap",            // QuickSwap (Polygon)
        401 => "SushiSwap",            // SushiSwapPolygon
        402 => "Curve",                // CurvePolygon
        404 => "Balancer",             // BalancerPolygon
        500 => "PancakeSwap",          // PancakeSwap (BSC)
        600 => "Uniswap V3",           // UniswapV3Arbitrum
        601 => "SushiSwap",            // SushiSwapArbitrum
        _ => &format!("DEX-{}", venue_a), // Show venue ID for unknown DEXs
    };

    let pool_b_venues = match venue_b {
        300 => "Uniswap V2",           // UniswapV2
        301 => "Uniswap V3",           // UniswapV3
        302 => "SushiSwap",            // SushiSwap (Ethereum)
        400 => "QuickSwap",            // QuickSwap (Polygon)
        401 => "SushiSwap",            // SushiSwapPolygon
        402 => "Curve",                // CurvePolygon
        404 => "Balancer",             // BalancerPolygon
        500 => "PancakeSwap",          // PancakeSwap (BSC)
        600 => "Uniswap V3",           // UniswapV3Arbitrum
        601 => "SushiSwap",            // SushiSwapArbitrum
        _ => &format!("DEX-{}", venue_b), // Show venue ID for unknown DEXs
    };

    // Pre-calculate Q64.64 values to avoid block expressions in json! macro
    let profit_f64 = expected_profit_q as f64 / (1u128 << 64) as f64;
    let capital_f64 = required_capital_q as f64 / (1u128 << 64) as f64;
    let amount_f64 = optimal_amount_q as f64 / (1u128 << 64) as f64;
    let _gas_cost_f64 = estimated_gas_cost_q as f64 / (1u128 << 64) as f64;

    // Calculate derived values
    let profit_percent = if capital_f64 > 0.0 { (profit_f64 / capital_f64) * 100.0 } else { 0.0 };
    let total_fees = capital_f64 * 0.006; // 0.6% total DEX fees
    let gas_cost_usd = (300000u64 * max_gas_price_gwei as u64) as f64 / 1e9 * 0.50; // Polygon MATIC ~$0.50
    let slippage_cost = capital_f64 * (slippage_tolerance as f64 / 10000.0);
    let net_profit = profit_f64 - total_fees - gas_cost_usd - slippage_cost;

    // Create comprehensive arbitrage opportunity JSON
    Ok(json!({
        "msg_type": "arbitrage_opportunity",
        "type": "demo_defi_arbitrage",
        "detected_at": timestamp_ns,
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),

        // Strategy Information
        "strategy_id": strategy_id,
        "strategy_name": get_strategy_name(strategy_id),
        "signal_id": signal_id.to_string(),
        "confidence_score": confidence,
        "chain_id": chain_id,
        "priority": priority,

        // Financial Metrics - Enhanced with precise calculations using Q64.64 fixed-point
        "estimated_profit": profit_f64,
        "net_profit_usd": profit_f64,
        "max_trade_size": capital_f64,
        "tradeSize": amount_f64,
        "grossProfit": profit_f64,
        "netProfit": net_profit,
        "profit_percent": profit_percent,
        "net_profit_percent": profit_percent,
        "optimal_trade_amount": DemoDeFiArbitrageTLV::q64_to_decimal_string(optimal_amount_q, 6), // Assume USDC (6 decimals)
        "gas_cost_estimate": DemoDeFiArbitrageTLV::q64_to_decimal_string(estimated_gas_cost_q, 18),

        // Enhanced Arbitrage Metrics for Trading View
        "arbitrage_metrics": {
            "spread_usd": profit_f64,
            "spread_percent": profit_percent,
            "optimal_size_usd": amount_f64,
            "dex_fees": {
                "pool_a_fee": 0.3, // Default 0.3% for V2
                "pool_b_fee": 0.3,
                "total_fee_usd": total_fees,
            },
            "gas_estimate": {
                "gas_units": 300000,
                "gas_price_gwei": max_gas_price_gwei,
                "cost_usd": gas_cost_usd,
            },
            "slippage_estimate": {
                "tolerance_bps": slippage_tolerance,
                "impact_usd": slippage_cost,
            },
            "net_calculation": {
                "gross_profit": profit_f64,
                "total_fees": total_fees,
                "gas_cost": gas_cost_usd,
                "slippage": slippage_cost,
                "net_profit": net_profit,
            },
            "executable": valid_until > (timestamp_ns / 1_000_000_000) as u32,
            "confidence_score": confidence,
        },

        // Pool Information with proper token mapping
        "pair": format!("{}/{}",
            map_token_symbol(token_in),
            map_token_symbol(token_out)
        ),
        "token_a": format!("0x{:016x}", token_in),
        "token_b": format!("0x{:016x}", token_out),
        "dex_buy": pool_a_venues,
        "dex_sell": pool_b_venues,
        "pool_a": format!("{:?}", pool_a),
        "pool_b": format!("{:?}", pool_b),
        "dex_buy_router": "0x0000000000000000000000000000000000000000", // Placeholder
        "dex_sell_router": "0x0000000000000000000000000000000000000000", // Placeholder

        // Trading Parameters
        "slippage_tolerance": format!("{:.2}%", slippage_tolerance as f64 / 100.0),
        "max_gas_price_gwei": max_gas_price_gwei,
        "valid_until": valid_until,
        "is_valid": valid_until > (timestamp_ns / 1_000_000_000) as u32,
        "executable": valid_until > (timestamp_ns / 1_000_000_000) as u32,

        // Dashboard compatibility values with proper calculations
        "price_buy": 0.0, // Pool prices not available in current TLV
        "price_sell": 0.0, // Pool prices not available in current TLV
        "buyPrice": 0.0,
        "sellPrice": 0.0,
        "spread": profit_percent,
        "gasFee": gas_cost_usd,
        "gas_fee_usd": gas_cost_usd,
        "dexFees": total_fees,
        "dex_fees_usd": total_fees,
        "slippage": slippage_cost,
        "slippage_cost_usd": slippage_cost,
        "netProfitPercent": profit_percent,
        "buyExchange": pool_a_venues,
        "sellExchange": pool_b_venues,
        "buyPool": format!("{:?}", pool_a),
        "sellPool": format!("{:?}", pool_b),

        // Raw TLV data for debugging
        "raw_data": {
            "strategy_id": strategy_id,
            "signal_id": signal_id,
            "confidence": confidence,
            "chain_id": chain_id,
            "expected_profit_q": expected_profit_q.to_string(),
            "required_capital_q": required_capital_q.to_string(),
            "estimated_gas_cost_q": estimated_gas_cost_q.to_string(),
            "token_in": format!("0x{:016x}", token_in),
            "token_out": format!("0x{:016x}", token_out),
            "optimal_amount_q": optimal_amount_q.to_string(),
            "slippage_tolerance": slippage_tolerance,
            "max_gas_price_gwei": max_gas_price_gwei,
            "valid_until": valid_until,
            "priority": priority,
            "timestamp_ns": timestamp_ns
        }
    }))
}

fn convert_pool_liquidity_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    // Simple placeholder for pool liquidity
    Ok(json!({
        "msg_type": "pool_liquidity",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len()
    }))
}

fn convert_pool_mint_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    // Simple placeholder for pool mint
    Ok(json!({
        "msg_type": "pool_mint",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len()
    }))
}

fn convert_pool_burn_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    // Simple placeholder for pool burn
    Ok(json!({
        "msg_type": "pool_burn",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len()
    }))
}

fn convert_pool_tick_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    // Simple placeholder for pool tick
    Ok(json!({
        "msg_type": "pool_tick",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len()
    }))
}

/// Helper function to convert sqrt_price bytes to string
fn convert_sqrt_price_to_string(sqrt_price_bytes: &[u8; 32]) -> String {
    // Convert first 16 bytes to u128 for display
    let mut price_bytes = [0u8; 16];
    price_bytes.copy_from_slice(&sqrt_price_bytes[..16]);
    let price_u128 = u128::from_le_bytes(price_bytes);
    if price_u128 > 0 {
        format!("{}", price_u128)
    } else {
        "0".to_string()
    }
}

fn convert_pool_swap_tlv(payload: &[u8], _timestamp_ns: u64) -> Result<Value> {
    let swap = PoolSwapTLV::from_bytes(payload).map_err(|_e| {
        DashboardError::Protocol(protocol_v2::ProtocolError::Parse(
            ParseError::MessageTooSmall { need: 32, got: 0 },
        ))
    })?;

    // Convert amounts to human-readable format using native decimals
    let amount_in_normalized = if swap.amount_in_decimals > 0 {
        swap.amount_in as f64 / 10_f64.powi(swap.amount_in_decimals as i32)
    } else {
        swap.amount_in as f64
    };

    let amount_out_normalized = if swap.amount_out_decimals > 0 {
        swap.amount_out as f64 / 10_f64.powi(swap.amount_out_decimals as i32)
    } else {
        swap.amount_out as f64
    };

    // Convert venue number to proper name
    let venue_name = match swap.venue {
        200 => "Ethereum",
        201 => "Bitcoin",
        202 => "Polygon",
        203 => "BSC",
        300 => "UniswapV2",
        301 => "UniswapV3",
        302 => "SushiSwap",
        _ => "Unknown",
    };

    Ok(json!({
        "msg_type": "pool_swap",
        "venue": swap.venue,
        "venue_name": venue_name,
        "pool_address": format!("0x{}", hex::encode(swap.pool_address)),
        "token_in": format!("0x{}", hex::encode(swap.token_in_addr)),
        "token_out": format!("0x{}", hex::encode(swap.token_out_addr)),
        "amount_in": {
            "raw": swap.amount_in.to_string(), // Use string to avoid JSON number limits
            "normalized": amount_in_normalized,
            "decimals": swap.amount_in_decimals
        },
        "amount_out": {
            "raw": swap.amount_out.to_string(), // Use string to avoid JSON number limits
            "normalized": amount_out_normalized,
            "decimals": swap.amount_out_decimals
        },
        // Protocol data - properly convert sqrt_price from bytes
        "sqrt_price_x96_after": convert_sqrt_price_to_string(&swap.sqrt_price_x96_after),
        "tick_after": swap.tick_after,
        "liquidity_after": swap.liquidity_after.to_string(),
        "timestamp": swap.timestamp_ns,
        "timestamp_iso": timestamp_to_iso(swap.timestamp_ns),
        "block_number": swap.block_number
    }))
}

/// Convert FlashLoanResult TLV (type 67)
fn convert_flash_loan_result_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    Ok(json!({
        "msg_type": "flash_loan_result",
        "tlv_type": 67,
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len(),
        "raw_data": base64::encode(payload)
    }))
}

/// Convert pool sync TLV (type 16) - V2 Sync events with complete reserves
fn convert_pool_sync_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    let sync = PoolSyncTLV::from_bytes(payload).map_err(|_e| {
        DashboardError::Protocol(protocol_v2::ProtocolError::Parse(
            ParseError::MessageTooSmall { need: 32, got: 0 },
        ))
    })?;

    // Convert reserves to normalized amounts (avoiding JSON number range issues)
    let reserve0_normalized = sync.reserve0 as f64 / 10_f64.powi(sync.token0_decimals as i32);
    let reserve1_normalized = sync.reserve1 as f64 / 10_f64.powi(sync.token1_decimals as i32);

    Ok(json!({
        "msg_type": "pool_sync",
        "venue": sync.venue as u16,
        "venue_name": format!("{:?}", sync.venue),
        "pool_address": format!("0x{}", hex::encode(sync.pool_address)),
        "token0_address": format!("0x{}", hex::encode(sync.token0_addr)),
        "token1_address": format!("0x{}", hex::encode(sync.token1_addr)),
        "reserves": {
            "reserve0": {
                "raw": sync.reserve0.to_string(), // Use string to avoid JSON number limits
                "normalized": reserve0_normalized,
                "decimals": sync.token0_decimals
            },
            "reserve1": {
                "raw": sync.reserve1.to_string(), // Use string to avoid JSON number limits
                "normalized": reserve1_normalized,
                "decimals": sync.token1_decimals
            }
        },
        "block_number": sync.block_number,
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "original_timestamp": sync.timestamp_ns,
        "original_timestamp_iso": timestamp_to_iso(sync.timestamp_ns)
    }))
}

/// Convert vendor proprietary data TLV (type 202)
fn convert_proprietary_data_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    Ok(json!({
        "msg_type": "proprietary_data",
        "tlv_type": 202,
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len(),
        "raw_data": base64::encode(payload)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn test_timestamp_conversion() {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let iso = timestamp_to_iso(now_ns);
        assert!(!iso.is_empty());
    }

    #[test]
    fn test_strategy_name_mapping() {
        assert_eq!(get_strategy_name(20), "Kraken Signals");
        assert_eq!(get_strategy_name(21), "Flash Arbitrage");
        assert_eq!(get_strategy_name(999), "Unknown Strategy");
    }
}
