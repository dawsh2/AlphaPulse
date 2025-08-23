use tokio::net::UnixStream;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    println!("🔍 TLV Corruption Debugger");
    println!("{}", "=".repeat(60));
    
    // Connect to relay
    let mut stream = UnixStream::connect("/tmp/alphapulse/market_data.sock")
        .await
        .expect("Failed to connect to relay");
        
    println!("✅ Connected to MarketDataRelay");
    println!();
    
    let mut buffer = vec![0u8; 8192];
    let mut msg_count = 0;
    
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                println!("❌ Connection closed");
                break;
            }
            Ok(bytes_read) => {
                msg_count += 1;
                println!("📨 Message #{}: {} bytes total", msg_count, bytes_read);
                
                // Parse header (32 bytes)
                if bytes_read >= 32 {
                    println!("  Header Analysis:");
                    
                    // Magic (bytes 0-3)
                    let magic = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                    println!("    Magic: 0x{:08x} {}", 
                           magic, 
                           if magic == 0xDEADBEEF { "✅" } else { "❌ CORRUPT!" });
                    
                    // Version (bytes 4-5)
                    let version = u16::from_le_bytes([buffer[4], buffer[5]]);
                    println!("    Version: {}", version);
                    
                    // Relay Domain (byte 6)
                    let domain = buffer[6];
                    println!("    Relay Domain: {} ({})", 
                           domain,
                           match domain {
                               1 => "MarketData",
                               2 => "OrderRouting",
                               3 => "RiskManagement",
                               _ => "Unknown"
                           });
                    
                    // Source Type (byte 7)
                    let source = buffer[7];
                    println!("    Source Type: {} ({})",
                           source,
                           match source {
                               1 => "DirectConnect",
                               2 => "RelayForward",
                               _ => "Unknown"
                           });
                    
                    // Payload Size (bytes 8-11)
                    let payload_size = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
                    println!("    Payload Size: {} bytes", payload_size);
                    
                    // Sequence (bytes 12-15)
                    let sequence = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
                    println!("    Sequence: {}", sequence);
                    
                    // Timestamp (bytes 16-23)
                    let timestamp = u64::from_le_bytes([
                        buffer[16], buffer[17], buffer[18], buffer[19],
                        buffer[20], buffer[21], buffer[22], buffer[23],
                    ]);
                    println!("    Timestamp: {} ns", timestamp);
                    
                    // Checksum (bytes 24-27)
                    let checksum = u32::from_le_bytes([buffer[24], buffer[25], buffer[26], buffer[27]]);
                    println!("    Checksum: 0x{:08x}", checksum);
                    
                    // Consumer ID (bytes 28-31)
                    let consumer_id = u32::from_le_bytes([buffer[28], buffer[29], buffer[30], buffer[31]]);
                    println!("    Consumer ID: {}", consumer_id);
                    
                    // TLV Payload Analysis
                    if bytes_read >= 32 + payload_size as usize {
                        println!("\n  TLV Payload Analysis:");
                        let tlv_data = &buffer[32..32 + payload_size as usize];
                        
                        // Print raw bytes of first 64 bytes of payload
                        println!("    Raw TLV bytes (first 64):");
                        for chunk in tlv_data.chunks(16).take(4) {
                            print!("      ");
                            for byte in chunk {
                                print!("{:02x} ", byte);
                            }
                            print!("  |");
                            for byte in chunk {
                                if *byte >= 32 && *byte <= 126 {
                                    print!("{}", *byte as char);
                                } else {
                                    print!(".");
                                }
                            }
                            println!("|");
                        }
                        
                        // Parse TLVs
                        println!("\n    TLV Messages:");
                        let mut offset = 0;
                        let mut tlv_count = 0;
                        
                        while offset + 2 <= tlv_data.len() {
                            tlv_count += 1;
                            let tlv_type = tlv_data[offset];
                            let tlv_length = tlv_data[offset + 1] as usize;
                            
                            println!("      TLV #{}: Type={} ({}), Length={}", 
                                   tlv_count,
                                   tlv_type,
                                   match tlv_type {
                                       1 => "TradeTLV",
                                       2 => "BestBidAskTLV", 
                                       3 => "OrderBookTLV",
                                       4 => "MarketStatusTLV",
                                       5 => "InstrumentInfoTLV",
                                       6 => "PriceLevelTLV",
                                       7 => "MarketStatsTLV",
                                       8 => "ImbalanceTLV",
                                       9 => "AuctionTLV",
                                       10 => "PoolLiquidityTLV",
                                       11 => "PoolSwapTLV",
                                       12 => "PoolMintTLV",
                                       13 => "PoolBurnTLV",
                                       14 => "PoolTickTLV",
                                       _ => "Unknown"
                                   },
                                   tlv_length);
                            
                            if offset + 2 + tlv_length > tlv_data.len() {
                                println!("        ⚠️ Incomplete TLV! Need {} bytes, have {}", 
                                       tlv_length, tlv_data.len() - offset - 2);
                                break;
                            }
                            
                            // Show first few bytes of payload
                            if tlv_length > 0 {
                                let payload_preview = &tlv_data[offset + 2..offset + 2 + tlv_length.min(16)];
                                print!("        Payload preview: ");
                                for byte in payload_preview {
                                    print!("{:02x} ", byte);
                                }
                                if tlv_length > 16 {
                                    print!("...");
                                }
                                println!();
                            }
                            
                            offset += 2 + tlv_length;
                        }
                        
                        if offset < tlv_data.len() {
                            println!("      ⚠️ {} trailing bytes after TLVs", tlv_data.len() - offset);
                        }
                    } else {
                        println!("  ⚠️ Incomplete payload: expected {} bytes, got {}", 
                               payload_size, bytes_read - 32);
                    }
                } else {
                    println!("  ❌ Message too small for header: {} bytes", bytes_read);
                }
                
                println!("{}", "=".repeat(60));
                
                if msg_count >= 3 {
                    println!("\n✅ Captured {} messages for analysis", msg_count);
                    break;
                }
            }
            Err(e) => {
                println!("❌ Read error: {}", e);
                break;
            }
        }
    }
}