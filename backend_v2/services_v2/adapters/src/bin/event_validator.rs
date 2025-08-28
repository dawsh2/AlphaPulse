#!/usr/bin/env rust-script
//! Event Signature Validator - Compares received events against canonical ABI
//!
//! Usage: cargo run --bin event_validator -- --ws-url wss://polygon-bor.publicnode.com
//!
//! This tool monitors raw WebSocket events and validates their signatures against
//! the canonical torq_dex event definitions to identify mismatches.

use anyhow::{Context, Result};
use clap::Parser;
use futures_util::StreamExt;
use serde_json::Value;
use std::collections::HashSet;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};
use web3::types::H256;

#[derive(Parser, Debug)]
#[clap(name = "event_validator")]
struct Args {
    /// WebSocket URL to connect to
    #[clap(long, default_value = "wss://polygon-bor.publicnode.com")]
    ws_url: String,

    /// Duration to monitor in seconds (0 for infinite)
    #[clap(long, default_value = "60")]
    duration: u64,

    /// Show all events, not just mismatches
    #[clap(long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!("🔍 Event Signature Validator Starting");
    info!("📡 Connecting to: {}", args.ws_url);

    // Get canonical event signatures from libs/dex
    let canonical_signatures = get_canonical_signatures();
    info!(
        "📚 Loaded {} canonical event signatures",
        canonical_signatures.len()
    );
    for sig in &canonical_signatures {
        info!("  ✓ {}", hex::encode(sig));
    }

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(&args.ws_url)
        .await
        .context("Failed to connect to WebSocket")?;
    info!("✅ Connected to WebSocket");

    // Subscribe to swap events
    let subscribe_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": [
            "logs",
            {
                "topics": [
                    canonical_signatures.iter()
                        .map(|s| format!("0x{}", hex::encode(s)))
                        .collect::<Vec<_>>()
                ]
            }
        ]
    });

    let (mut write, mut read) = ws_stream.split();

    use futures_util::SinkExt;
    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .context("Failed to send subscription")?;
    info!("📨 Subscription sent for DEX events");

    // Track statistics
    let mut total_events = 0u64;
    let mut matched_events = 0u64;
    let mut unknown_signatures = HashSet::new();

    let start_time = std::time::Instant::now();
    let duration = if args.duration == 0 {
        std::time::Duration::from_secs(u64::MAX)
    } else {
        std::time::Duration::from_secs(args.duration)
    };

    info!("👂 Listening for events...\n");

    while let Some(msg) = read.next().await {
        if start_time.elapsed() > duration {
            break;
        }

        match msg? {
            Message::Text(text) => {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    // Handle subscription confirmation
                    if json.get("result").is_some() {
                        info!("✅ Subscription confirmed");
                        continue;
                    }

                    // Process event
                    if let Some(params) = json.get("params") {
                        if let Some(result) = params.get("result") {
                            if let Some(topics) = result.get("topics").and_then(|t| t.as_array()) {
                                if let Some(first_topic) = topics.first().and_then(|t| t.as_str()) {
                                    total_events += 1;

                                    // Parse event signature
                                    let sig_hex = first_topic.trim_start_matches("0x");
                                    if let Ok(sig_bytes) = hex::decode(sig_hex) {
                                        if sig_bytes.len() == 32 {
                                            let mut sig_array = [0u8; 32];
                                            sig_array.copy_from_slice(&sig_bytes);

                                            // Check against canonical signatures
                                            if canonical_signatures.contains(&sig_array) {
                                                matched_events += 1;
                                                if args.verbose {
                                                    info!("✅ Known event: 0x{}", sig_hex);
                                                }
                                            } else {
                                                unknown_signatures.insert(sig_hex.to_string());
                                                warn!("❌ UNKNOWN EVENT SIGNATURE: 0x{}", sig_hex);

                                                // Try to decode event data for debugging
                                                if let Some(data) =
                                                    result.get("data").and_then(|d| d.as_str())
                                                {
                                                    warn!("   Data: {}", data);
                                                }
                                                if let Some(address) =
                                                    result.get("address").and_then(|a| a.as_str())
                                                {
                                                    warn!("   Contract: {}", address);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Message::Close(_) => {
                info!("WebSocket connection closed");
                break;
            }
            _ => {}
        }
    }

    // Print summary
    println!("\n📊 Validation Summary");
    println!("═══════════════════════════════════════");
    println!("Total events received:  {}", total_events);
    println!(
        "Matched canonical:      {} ({:.1}%)",
        matched_events,
        if total_events > 0 {
            (matched_events as f64 / total_events as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("Unknown signatures:     {}", unknown_signatures.len());

    if !unknown_signatures.is_empty() {
        println!("\n⚠️  Unknown Event Signatures:");
        for sig in unknown_signatures.iter().take(10) {
            println!("  - 0x{}", sig);
        }
        if unknown_signatures.len() > 10 {
            println!("  ... and {} more", unknown_signatures.len() - 10);
        }

        println!("\n💡 These signatures are not in torq_dex!");
        println!("   Either they're from different protocols or the ABI needs updating.");
    } else {
        println!("\n✅ All received events match canonical signatures!");
    }

    Ok(())
}

/// Convert hex string to 32-byte array
fn hex_to_bytes32(hex_str: &str) -> Option<[u8; 32]> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    if hex_str.len() != 64 {
        return None;
    }

    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let hex_byte = &hex_str[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(hex_byte, 16).ok()?;
    }
    Some(bytes)
}

/// Get canonical event signatures from libs/dex
fn get_canonical_signatures() -> HashSet<[u8; 32]> {
    let mut signatures = HashSet::new();

    // Get all event signatures from the canonical source
    let all_sigs = torq_dex::get_all_event_signatures();
    for sig in all_sigs {
        if let Some(bytes) = hex_to_bytes32(&sig) {
            signatures.insert(bytes);
        }
    }

    // Also specifically get swap signatures for validation
    let (v2_swap, v3_swap) = torq_dex::get_swap_signatures();
    if let Some(bytes) = hex_to_bytes32(&v2_swap) {
        signatures.insert(bytes);
    }
    if let Some(bytes) = hex_to_bytes32(&v3_swap) {
        signatures.insert(bytes);
    }

    signatures
}
