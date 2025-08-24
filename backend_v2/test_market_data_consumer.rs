#!/usr/bin/env rust-script
//! Test consumer to read Polygon events from market data relay

use protocol_v2::{parse_header, parse_tlv_extensions, TLVExtensionEnum, TLVType};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 Connecting to market data relay to display Polygon events\n");

    let socket_path = "/tmp/alphapulse/market_data.sock";
    let mut stream = match UnixStream::connect(socket_path).await {
        Ok(stream) => {
            println!("✅ Connected to market data relay at {}", socket_path);
            stream
        }
        Err(e) => {
            println!("❌ Failed to connect to {}: {}", socket_path, e);
            println!("   Make sure the market data relay is running");
            return Err(e.into());
        }
    };

    // Send consumer registration (simple handshake)
    let handshake = b"CONSUMER";
    stream.write_all(handshake).await?;
    println!("📝 Sent consumer registration");

    let mut event_count = 0u32;
    let mut buffer = vec![0u8; 8192]; // Buffer for reading messages
    let start_time = std::time::Instant::now();

    println!("👂 Listening for Polygon DEX events...\n");

    loop {
        // Read with timeout
        match tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buffer)).await {
            Ok(Ok(n)) if n > 0 => {
                // Parse the message
                match parse_message(&buffer[..n]) {
                    Ok(info) => {
                        event_count += 1;
                        print_event_info(event_count, &info);

                        // Show first 10 events in detail, then every 10th
                        if event_count >= 10 && event_count % 10 != 0 {
                            continue;
                        }
                    }
                    Err(e) => {
                        println!("⚠️ Failed to parse message: {}", e);
                    }
                }
            }
            Ok(Ok(_)) => {
                println!("📡 Connection closed by relay");
                break;
            }
            Ok(Err(e)) => {
                println!("❌ Read error: {}", e);
                break;
            }
            Err(_) => {
                // Timeout - show status
                if event_count == 0 {
                    print!(".");
                } else {
                    let elapsed = start_time.elapsed().as_secs();
                    let rate = if elapsed > 0 {
                        event_count as f64 / elapsed as f64
                    } else {
                        0.0
                    };
                    println!(
                        "📊 Status: {} events received, rate: {:.1} events/sec",
                        event_count, rate
                    );
                }
            }
        }

        // Stop after reasonable time for demonstration
        if start_time.elapsed() > Duration::from_secs(60) {
            println!("\n⏱️ Test complete after 60 seconds");
            break;
        }
    }

    println!("\n🏁 Final stats:");
    println!("   Total events: {}", event_count);
    println!("   Duration: {:.1}s", start_time.elapsed().as_secs_f64());
    println!(
        "   Average rate: {:.2} events/sec",
        event_count as f64 / start_time.elapsed().as_secs_f64()
    );

    Ok(())
}

fn parse_message(data: &[u8]) -> Result<EventInfo, Box<dyn std::error::Error>> {
    if data.len() < 32 {
        return Err("Message too short for header".into());
    }

    let header = parse_header(data)?;

    if data.len() < 32 + header.payload_size as usize {
        return Err("Message shorter than expected payload".into());
    }

    let payload = &data[32..32 + header.payload_size as usize];
    let tlvs = parse_tlv_extensions(payload)?;

    let mut event_info = EventInfo {
        sequence: header.sequence as u32,
        source: header.source,
        timestamp: header.timestamp,
        tlv_count: tlvs.len(),
        event_types: Vec::new(),
    };

    for tlv in tlvs {
        let tlv_type_num = match tlv {
            TLVExtensionEnum::Standard(std_tlv) => std_tlv.header.tlv_type,
            TLVExtensionEnum::Extended(ext_tlv) => ext_tlv.header.tlv_type,
        };

        if let Ok(tlv_type) = TLVType::try_from(tlv_type_num) {
            event_info.event_types.push(format!("{:?}", tlv_type));
        } else {
            event_info
                .event_types
                .push(format!("Unknown({})", tlv_type_num));
        }
    }

    Ok(event_info)
}

#[derive(Debug)]
struct EventInfo {
    sequence: u32,
    source: u8,
    timestamp: u64,
    tlv_count: usize,
    event_types: Vec<String>,
}

fn print_event_info(count: u32, info: &EventInfo) {
    println!("🎯 Event #{}", count);
    println!("   Sequence: {}", info.sequence);
    println!("   Source: {}", info.source);
    println!("   Timestamp: {} ns", info.timestamp);
    println!("   TLV Count: {}", info.tlv_count);
    println!("   Types: {}", info.event_types.join(", "));
    println!();
}
