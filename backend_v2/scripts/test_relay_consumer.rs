use tokio::net::UnixStream;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    println!("🔌 Connecting to market data relay...");
    
    let mut stream = UnixStream::connect("/tmp/alphapulse/market_data.sock")
        .await
        .expect("Failed to connect");
        
    println!("✅ Connected! Waiting for messages...");
    
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
                println!("📨 Message #{}: {} bytes", msg_count, bytes);
                
                // Print first 32 bytes as hex
                if bytes >= 32 {
                    let header_hex: String = buffer[..32]
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!("   Header: {}", header_hex);
                    
                    // Check magic
                    let magic = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                    println!("   Magic: 0x{:08x}", magic);
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
    
    println!("Total messages received: {}", msg_count);
}