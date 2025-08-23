// Example client showing how to properly consume messages from the relay

use alphapulse_protocol::{MessageHeader, MESSAGE_MAGIC};
use std::io::Read;
use std::os::unix::net::UnixStream;

const RELAY_PATH: &str = "/tmp/alphapulse/relay.sock";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to relay at {}", RELAY_PATH);
    let mut stream = UnixStream::connect(RELAY_PATH)?;
    
    let mut pending_data = Vec::new();
    let mut buffer = vec![0u8; 65536];
    
    println!("Connected! Listening for messages...");
    
    loop {
        // Read available data
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("Relay disconnected");
                break;
            }
            Ok(n) => {
                // Append to pending data
                pending_data.extend_from_slice(&buffer[..n]);
                
                // Process complete messages
                while pending_data.len() >= std::mem::size_of::<MessageHeader>() {
                    // Parse header to get message size
                    let header_bytes = &pending_data[..std::mem::size_of::<MessageHeader>()];
                    
                    // Check magic number
                    let magic = u32::from_le_bytes([
                        header_bytes[0], header_bytes[1], header_bytes[2], header_bytes[3]
                    ]);
                    
                    if magic != MESSAGE_MAGIC {
                        eprintln!("Invalid magic number: 0x{:08x}", magic);
                        // Try to resync by finding next 0xDEADBEEF
                        if let Some(pos) = pending_data[1..].windows(4)
                            .position(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]) == MESSAGE_MAGIC) 
                        {
                            pending_data.drain(..pos + 1);
                            continue;
                        } else {
                            // No valid magic found, clear buffer
                            pending_data.clear();
                            break;
                        }
                    }
                    
                    // Parse header to get payload size
                    if let Ok(header) = MessageHeader::from_bytes(header_bytes) {
                        let total_size = std::mem::size_of::<MessageHeader>() + header.payload_size as usize;
                        
                        if pending_data.len() >= total_size {
                            // Extract complete message
                            let message = pending_data[..total_size].to_vec();
                            pending_data.drain(..total_size);
                            
                            // Process the message
                            process_message(&message)?;
                        } else {
                            // Wait for more data
                            break;
                        }
                    } else {
                        eprintln!("Failed to parse header");
                        // Skip this byte and try again
                        pending_data.drain(..1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

fn process_message(message: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // Parse header
    let header = MessageHeader::from_bytes(&message[..std::mem::size_of::<MessageHeader>()])?;
    
    println!("Received message: type={}, size={}, source={}, timestamp={}", 
        header.message_type, 
        header.payload_size,
        header.source,
        header.timestamp
    );
    
    // Here you would parse the specific message type based on header.message_type
    // and process accordingly
    
    Ok(())
}