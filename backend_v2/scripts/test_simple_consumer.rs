use tokio::net::UnixStream;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    println!("🔌 Simple TLV Consumer Test");
    
    let mut stream = UnixStream::connect("/tmp/alphapulse/market_data.sock")
        .await
        .expect("Failed to connect");
        
    println!("✅ Connected to relay");
    
    let mut buffer = vec![0u8; 8192];
    let mut msg_count = 0;
    
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                println!("Connection closed");
                break;
            }
            Ok(bytes) => {
                msg_count += 1;
                println!("\n📨 Message #{}: {} bytes", msg_count, bytes);
                
                // Parse header  
                if bytes >= 32 {
                    let magic = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                    let domain = buffer[4];
                    let version = buffer[5]; 
                    let source = buffer[6];
                    let flags = buffer[7];
                    let payload_size = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
                    
                    println!("  Magic: 0x{:08x} {}", magic, if magic == 0xDEADBEEF { "✅" } else { "❌" });
                    println!("  Domain: {}, Version: {}, Source: {}, Flags: {}", domain, version, source, flags);
                    println!("  Payload size: {} bytes", payload_size);
                    
                    // Parse TLVs
                    if bytes >= 32 + payload_size as usize {
                        let tlv_data = &buffer[32..32 + payload_size as usize];
                        let mut offset = 0;
                        let mut tlv_num = 0;
                        
                        while offset + 2 <= tlv_data.len() {
                            tlv_num += 1;
                            let tlv_type = tlv_data[offset];
                            let tlv_length = tlv_data[offset + 1] as usize;
                            
                            println!("  TLV #{}: Type={}, Length={}", tlv_num, tlv_type, tlv_length);
                            
                            if offset + 2 + tlv_length > tlv_data.len() {
                                println!("    ⚠️ Incomplete TLV");
                                break;
                            }
                            
                            offset += 2 + tlv_length;
                        }
                    }
                }
                
                if msg_count >= 3 {
                    println!("\n✅ Test complete - received {} messages", msg_count);
                    break;
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
}