//! TLV to JSON message conversion for dashboard

use crate::error::{DashboardError, Result};
use alphapulse_types::InstrumentId;
use alphapulse_types::{
    tlv::{types::DeprecatedTLVType, ArbitrageSignalTLV, ParseError, PoolSwapTLV, PoolSyncTLV},
    QuoteTLV, VenueId,
};
use base64::prelude::*;
use serde_json::{json, Value};
use torq_network::time::safe_system_timestamp_ns_checked;

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
        // Handle deprecated types with proper enum
        _ => {
            // Check if this is a deprecated type
            if let Some(deprecation_msg) = DeprecatedTLVType::deprecation_message(tlv_type) {
                Ok(json!({
                    "msg_type": "deprecated",
                    "error": deprecation_msg,
                    "timestamp": timestamp_ns
                }))
            } else {
                Ok(json!({
                    "msg_type": "unknown",
                    "tlv_type": tlv_type,
                    "timestamp": timestamp_ns,
                    "raw_data": base64::prelude::BASE64_STANDARD.encode(payload)
                }))
            }
        }
    }
}

fn convert_trade_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    if payload.len() < 22 {
        return Err(DashboardError::Protocol(
            alphapulse_types::protocol::ProtocolError::Parse(ParseError::MessageTooSmall {
                need: 22,
                got: 0,
            }),
        ));
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
        DashboardError::Protocol(alphapulse_types::protocol::ProtocolError::Parse(
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
        return Err(DashboardError::Protocol(
            alphapulse_types::protocol::ProtocolError::Parse(ParseError::MessageTooSmall {
                need: 12,
                got: payload.len(),
            }),
        ));
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
        match std::time::SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(timestamp_secs)) {
            Some(dt) => dt,
            None => {
                // Use safe timestamp function as fallback
                let safe_ns = safe_system_timestamp_ns_checked().unwrap_or(timestamp_ns);
                let safe_secs = safe_ns / 1_000_000_000;
                std::time::SystemTime::UNIX_EPOCH
                    .checked_add(std::time::Duration::from_secs(safe_secs))
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            }
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
        use crate::constants::defaults;
        opportunity["pair"] = json!(defaults::UNKNOWN_PAIR);
        opportunity["token_a"] = json!(defaults::DEFAULT_TOKEN_ADDRESS);
        opportunity["token_b"] = json!(defaults::DEFAULT_TOKEN_ADDRESS);
        opportunity["dex_buy"] = json!(defaults::DEFAULT_BUY_DEX);
        opportunity["dex_sell"] = json!(defaults::DEFAULT_SELL_DEX);
        opportunity["dex_buy_router"] = json!(defaults::DEFAULT_ROUTER_ADDRESS);
        opportunity["dex_sell_router"] = json!(defaults::DEFAULT_ROUTER_ADDRESS);
        opportunity["price_buy"] = json!(0.0);
        opportunity["price_sell"] = json!(0.0);
        opportunity["gas_fee_usd"] = json!(defaults::DEFAULT_GAS_FEE_USD);
        opportunity["dex_fees_usd"] = json!(defaults::DEFAULT_DEX_FEES_USD);
        opportunity["slippage_cost_usd"] = json!(defaults::DEFAULT_SLIPPAGE_COST_USD);
    }

    opportunity
}

/// Convert ArbitrageSignalTLV to arbitrage opportunity JSON
fn convert_arbitrage_signal_tlv(payload: &[u8], timestamp_ns: u64) -> Result<Value> {
    let signal = ArbitrageSignalTLV::from_bytes(payload).map_err(|_| {
        DashboardError::Protocol(alphapulse_types::protocol::ProtocolError::Parse(
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
    crate::constants::get_token_symbol(token_id)
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
        DashboardError::Protocol(alphapulse_types::protocol::ProtocolError::Parse(
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
        DashboardError::Protocol(alphapulse_types::protocol::ProtocolError::Parse(
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
        let now_ns =
            torq_network::time::safe_system_timestamp_ns_checked().unwrap_or(1000000000); // Use a fixed timestamp for test

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
