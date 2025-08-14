// Minimal test program to reproduce SIGBUS crash in shared memory
use alphapulse_common::{
    shared_memory::{SharedMemoryReader, OrderBookDeltaReader},
};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use memmap2::MmapOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Starting shared memory debug test...");
    
    // Test 1: Check if shared memory files exist
    let trade_file = "/tmp/alphapulse_shm/trades";
    let orderbook_file = "/tmp/alphapulse_shm/orderbook_deltas";
    
    println!("\n📂 Checking shared memory files:");
    
    if std::path::Path::new(trade_file).exists() {
        let metadata = std::fs::metadata(trade_file)?;
        println!("✅ Trades file exists: {} bytes", metadata.len());
    } else {
        println!("❌ Trades file does not exist: {}", trade_file);
    }
    
    if std::path::Path::new(orderbook_file).exists() {
        let metadata = std::fs::metadata(orderbook_file)?;
        println!("✅ Orderbook deltas file exists: {} bytes", metadata.len());
    } else {
        println!("❌ Orderbook deltas file does not exist: {}", orderbook_file);
    }
    
    // Test 2: Raw file inspection
    println!("\n🔍 Raw file header inspection:");
    
    if let Ok(mut file) = File::open(trade_file) {
        let mut header_buffer = [0u8; 256];
        match file.read(&mut header_buffer) {
            Ok(bytes_read) => {
                println!("Read {} bytes from trades file header:", bytes_read);
                println!("First 32 bytes: {:02x?}", &header_buffer[..32.min(bytes_read)]);
                
                // Try to interpret as header struct
                if bytes_read >= 24 {
                    let version = u32::from_le_bytes([header_buffer[0], header_buffer[1], header_buffer[2], header_buffer[3]]);
                    let capacity = u32::from_le_bytes([header_buffer[4], header_buffer[5], header_buffer[6], header_buffer[7]]);
                    let write_seq = u64::from_le_bytes([
                        header_buffer[8], header_buffer[9], header_buffer[10], header_buffer[11],
                        header_buffer[12], header_buffer[13], header_buffer[14], header_buffer[15]
                    ]);
                    let cached_seq = u64::from_le_bytes([
                        header_buffer[16], header_buffer[17], header_buffer[18], header_buffer[19],
                        header_buffer[20], header_buffer[21], header_buffer[22], header_buffer[23]
                    ]);
                    
                    println!("  Header interpretation:");
                    println!("    Version: {}", version);
                    println!("    Capacity: {}", capacity);
                    println!("    Write sequence: {}", write_seq);
                    println!("    Cached sequence: {}", cached_seq);
                }
            }
            Err(e) => {
                println!("❌ Failed to read trades file: {}", e);
            }
        }
    } else {
        println!("❌ Cannot open trades file for reading");
    }
    
    // Test 3: Memory mapping test (safer approach)
    println!("\n🗺️  Memory mapping test:");
    
    match OpenOptions::new().read(true).open(trade_file) {
        Ok(file) => {
            let file_len = file.metadata()?.len();
            println!("  File length: {} bytes", file_len);
            
            if file_len > 0 {
                match unsafe { MmapOptions::new().len(file_len as usize).map(&file) } {
                    Ok(mmap) => {
                        println!("✅ Memory mapping successful");
                        println!("  Mapped size: {} bytes", mmap.len());
                        println!("  Pointer: {:p}", mmap.as_ptr());
                        
                        // Check pointer alignment
                        let ptr_addr = mmap.as_ptr() as usize;
                        println!("  Pointer alignment:");
                        println!("    Address: 0x{:x}", ptr_addr);
                        println!("    Aligned to 8 bytes: {}", ptr_addr % 8 == 0);
                        println!("    Aligned to 16 bytes: {}", ptr_addr % 16 == 0);
                        println!("    Aligned to 32 bytes: {}", ptr_addr % 32 == 0);
                        println!("    Aligned to 64 bytes: {}", ptr_addr % 64 == 0);
                    }
                    Err(e) => {
                        println!("❌ Memory mapping failed: {}", e);
                    }
                }
            } else {
                println!("❌ File is empty");
            }
        }
        Err(e) => {
            println!("❌ Cannot open file: {}", e);
        }
    }
    
    // Test 4: Try to create SharedMemoryReader (this might crash)
    println!("\n🎯 Attempting to create SharedMemoryReader (this might cause SIGBUS)...");
    
    println!("  Creating reader for trades with reader_id=10...");
    match SharedMemoryReader::open(trade_file, 10) {
        Ok(mut reader) => {
            println!("✅ SharedMemoryReader created successfully!");
            
            println!("  Checking reader lag...");
            let lag = reader.get_lag();
            println!("  Reader lag: {}", lag);
            
            println!("  Attempting to read trades (this is where SIGBUS might occur)...");
            let trades = reader.read_trades();
            println!("✅ Successfully read {} trades", trades.len());
            
            if !trades.is_empty() {
                let first_trade = &trades[0];
                println!("  First trade:");
                println!("    Timestamp: {}", first_trade.timestamp_ns);
                println!("    Symbol: {}", first_trade.symbol_str());
                println!("    Exchange: {}", first_trade.exchange_str());
                println!("    Price: {}", first_trade.price);
                println!("    Volume: {}", first_trade.volume);
            }
        }
        Err(e) => {
            println!("❌ Failed to create SharedMemoryReader: {}", e);
        }
    }
    
    // Test 5: Try OrderBook deltas (might also crash)
    println!("\n📊 Attempting to create OrderBookDeltaReader...");
    
    match OrderBookDeltaReader::open(orderbook_file, 10) {
        Ok(mut reader) => {
            println!("✅ OrderBookDeltaReader created successfully!");
            
            println!("  Checking reader lag...");
            let lag = reader.get_lag();
            println!("  Reader lag: {}", lag);
            
            println!("  Attempting to read deltas (this is where SIGBUS might occur)...");
            let deltas = reader.read_deltas();
            println!("✅ Successfully read {} deltas", deltas.len());
            
            if !deltas.is_empty() {
                let first_delta = &deltas[0];
                println!("  First delta:");
                println!("    Timestamp: {}", first_delta.timestamp_ns);
                println!("    Symbol: {}", first_delta.symbol_str());
                println!("    Exchange: {}", first_delta.exchange_str());
                println!("    Version: {}", first_delta.version);
                println!("    Change count: {}", first_delta.change_count);
            }
        }
        Err(e) => {
            println!("❌ Failed to create OrderBookDeltaReader: {}", e);
        }
    }
    
    println!("\n✅ Test completed without SIGBUS!");
    Ok(())
}