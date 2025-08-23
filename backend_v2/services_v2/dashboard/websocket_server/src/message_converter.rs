//! TLV to JSON message conversion for dashboard

use crate::error::{DashboardError, Result};
use protocol_v2::InstrumentId;
use protocol_v2::{
    tlv::DemoDeFiArbitrageTLV, ParseError, PoolSwapTLV, QuoteTLV, StateInvalidationTLV,
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
        67 => convert_flash_loan_result_tlv(payload, timestamp_ns), // TLVType::FlashLoanResult
        202 => convert_proprietary_data_tlv(payload, timestamp_ns), // VendorTLVType::ProprietaryData
        255 => convert_demo_defi_arbitrage_tlv(payload, timestamp_ns), // DemoDeFiArbitrageTLV
        _ => Ok(json!({
            "type": "unknown",
            "tlv_type": tlv_type,
            "timestamp": timestamp_ns,
            "raw_data": base64::encode(payload)
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
        "type": "trade",
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
        "type": "quote",
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
        "type": "state_invalidation",
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
        "type": "trading_signal",
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

/// Convert DemoDeFiArbitrageTLV to arbitrage opportunity JSON
fn convert_demo_defi_arbitrage_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    let arbitrage_tlv = DemoDeFiArbitrageTLV::from_bytes(payload).map_err(|e| {
        DashboardError::Protocol(protocol_v2::ProtocolError::Parse(
            ParseError::MessageTooSmall {
                need: 124,
                got: payload.len(),
            },
        ))
    })?;

    // Extract pool information
    let pool_a_venues = match arbitrage_tlv.venue_a {
        protocol_v2::VenueId::UniswapV2 => "Uniswap V2",
        protocol_v2::VenueId::UniswapV3 => "Uniswap V3",
        protocol_v2::VenueId::SushiSwap => "SushiSwap V2",
        _ => "Unknown DEX",
    };

    let pool_b_venues = match arbitrage_tlv.venue_b {
        protocol_v2::VenueId::UniswapV2 => "Uniswap V2",
        protocol_v2::VenueId::UniswapV3 => "Uniswap V3",
        protocol_v2::VenueId::SushiSwap => "SushiSwap V2",
        _ => "Unknown DEX",
    };

    // Create comprehensive arbitrage opportunity JSON
    Ok(json!({
        "msg_type": "arbitrage_opportunity",
        "type": "demo_defi_arbitrage",
        "detected_at": timestamp_ns,
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),

        // Strategy Information
        "strategy_id": arbitrage_tlv.strategy_id,
        "strategy_name": get_strategy_name(arbitrage_tlv.strategy_id),
        "signal_id": arbitrage_tlv.signal_id.to_string(),
        "confidence_score": arbitrage_tlv.confidence,
        "chain_id": arbitrage_tlv.chain_id,
        "priority": arbitrage_tlv.priority,

        // Financial Metrics
        "estimated_profit": arbitrage_tlv.expected_profit_usd(),
        "net_profit_usd": arbitrage_tlv.expected_profit_usd(),
        "max_trade_size": arbitrage_tlv.required_capital_usd(),
        "profit_percent": if arbitrage_tlv.required_capital_q > 0 {
            ((arbitrage_tlv.expected_profit_q as f64 / arbitrage_tlv.required_capital_q as f64) * 100.0)
        } else { 0.0 },
        "net_profit_percent": if arbitrage_tlv.required_capital_q > 0 {
            ((arbitrage_tlv.expected_profit_q as f64 / arbitrage_tlv.required_capital_q as f64) * 100.0)
        } else { 0.0 },
        "optimal_trade_amount": arbitrage_tlv.optimal_amount_token(6), // Assume USDC (6 decimals)
        "gas_cost_estimate": arbitrage_tlv.estimated_gas_cost_native(),

        // Pool Information
        "pair": format!("0x{:016x}-0x{:016x}", arbitrage_tlv.token_in, arbitrage_tlv.token_out),
        "token_a": format!("0x{:016x}", arbitrage_tlv.token_in),
        "token_b": format!("0x{:016x}", arbitrage_tlv.token_out),
        "dex_buy": pool_a_venues,
        "dex_sell": pool_b_venues,
        "pool_a": format!("{:?}", arbitrage_tlv.pool_a),
        "pool_b": format!("{:?}", arbitrage_tlv.pool_b),
        "dex_buy_router": "0x0000000000000000000000000000000000000000", // Placeholder
        "dex_sell_router": "0x0000000000000000000000000000000000000000", // Placeholder

        // Trading Parameters
        "slippage_tolerance": arbitrage_tlv.slippage_percentage(),
        "max_gas_price_gwei": arbitrage_tlv.max_gas_price_gwei,
        "valid_until": arbitrage_tlv.valid_until,
        "is_valid": arbitrage_tlv.is_valid((timestamp_ns / 1_000_000_000) as u32),
        "executable": arbitrage_tlv.is_valid((timestamp_ns / 1_000_000_000) as u32),

        // Placeholder values for dashboard compatibility
        "price_buy": 0.0,
        "price_sell": 0.0,
        "gas_fee_usd": format!("{:.6}", arbitrage_tlv.estimated_gas_cost_native().parse::<f64>().unwrap_or(0.0)),
        "dex_fees_usd": 3.0,
        "slippage_cost_usd": 1.0,

        // Raw TLV data for debugging
        "raw_data": {
            "strategy_id": arbitrage_tlv.strategy_id,
            "signal_id": arbitrage_tlv.signal_id,
            "confidence": arbitrage_tlv.confidence,
            "chain_id": arbitrage_tlv.chain_id,
            "expected_profit_q": arbitrage_tlv.expected_profit_q.to_string(),
            "required_capital_q": arbitrage_tlv.required_capital_q.to_string(),
            "estimated_gas_cost_q": arbitrage_tlv.estimated_gas_cost_q.to_string(),
            "token_in": format!("0x{:016x}", arbitrage_tlv.token_in),
            "token_out": format!("0x{:016x}", arbitrage_tlv.token_out),
            "optimal_amount_q": arbitrage_tlv.optimal_amount_q.to_string(),
            "slippage_tolerance": arbitrage_tlv.slippage_tolerance,
            "max_gas_price_gwei": arbitrage_tlv.max_gas_price_gwei,
            "valid_until": arbitrage_tlv.valid_until,
            "priority": arbitrage_tlv.priority,
            "timestamp_ns": arbitrage_tlv.timestamp_ns
        }
    }))
}

fn convert_pool_liquidity_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    // Simple placeholder for pool liquidity
    Ok(json!({
        "type": "pool_liquidity",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len()
    }))
}

fn convert_pool_mint_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    // Simple placeholder for pool mint
    Ok(json!({
        "type": "pool_mint",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len()
    }))
}

fn convert_pool_burn_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    // Simple placeholder for pool burn
    Ok(json!({
        "type": "pool_burn",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len()
    }))
}

fn convert_pool_tick_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    // Simple placeholder for pool tick
    Ok(json!({
        "type": "pool_tick",
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len()
    }))
}

fn convert_pool_swap_tlv(payload: &[u8], _timestamp_ns: u64) -> Result<Value> {
    let swap = PoolSwapTLV::from_bytes(payload).map_err(|e| {
        DashboardError::Protocol(protocol_v2::ProtocolError::Parse(
            ParseError::MessageTooSmall { need: 32, got: 0 },
        ))
    })?;

    // Convert amounts to human-readable format using native decimals
    let amount_in_normalized = swap.amount_in as f64 / 10_f64.powi(swap.amount_in_decimals as i32);
    let amount_out_normalized =
        swap.amount_out as f64 / 10_f64.powi(swap.amount_out_decimals as i32);

    Ok(json!({
        "type": "pool_swap",
        "venue": swap.venue as u16,
        "venue_name": format!("{:?}", swap.venue),
        "pool_address": format!("0x{}", hex::encode(swap.pool_address)),
        "token_in": format!("0x{}", hex::encode(swap.token_in_addr)),
        "token_out": format!("0x{}", hex::encode(swap.token_out_addr)),
        "amount_in": {
            "raw": swap.amount_in,
            "normalized": amount_in_normalized,
            "decimals": swap.amount_in_decimals
        },
        "amount_out": {
            "raw": swap.amount_out,
            "normalized": amount_out_normalized,
            "decimals": swap.amount_out_decimals
        },
        "sqrt_price_x96_after": swap.sqrt_price_x96_after,
        "tick_after": swap.tick_after,
        "liquidity_after": swap.liquidity_after,
        "timestamp": swap.timestamp_ns,
        "timestamp_iso": timestamp_to_iso(swap.timestamp_ns),
        "block_number": swap.block_number
    }))
}

/// Convert FlashLoanResult TLV (type 67)
fn convert_flash_loan_result_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    Ok(json!({
        "type": "flash_loan_result",
        "tlv_type": 67,
        "timestamp": timestamp_ns,
        "timestamp_iso": timestamp_to_iso(timestamp_ns),
        "payload_size": payload.len(),
        "raw_data": base64::encode(payload)
    }))
}

/// Convert vendor proprietary data TLV (type 202)
fn convert_proprietary_data_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    Ok(json!({
        "type": "proprietary_data",
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
