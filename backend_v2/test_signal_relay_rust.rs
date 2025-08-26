//! Standalone test for Rust signal relay

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::time::Duration;
use std::thread;

fn create_test_message(sequence: u64) -> Vec<u8> {
    // Simple Protocol V2 message with ArbitrageSignalTLV
    let mut message = Vec::new();

    // Header (32 bytes)
    message.extend_from_slice(&0xDEADBEEFu32.to_le_bytes()); // magic
    message.push(2); // domain (Signal)
    message.push(4); // source (ArbitrageStrategy)
    message.extend_from_slice(&0u16.to_le_bytes()); // reserved
    message.extend_from_slice(&sequence.to_le_bytes()); // sequence
    let timestamp_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    message.extend_from_slice(&timestamp_ns.to_le_bytes()); // timestamp

    // Payload: TLV type 21 (ArbitrageSignalTLV)
    let mut payload = Vec::new();
    payload.extend_from_slice(&21u16.to_le_bytes()); // TLV type
    payload.extend_from_slice(&180u16.to_le_bytes()); // TLV length

    // ArbitrageSignalTLV data (180 bytes)
    let mut tlv_data = vec![0u8; 180];
    tlv_data[0..2].copy_from_slice(&21u16.to_le_bytes()); // strategy_id
    tlv_data[2..10].copy_from_slice(&sequence.to_le_bytes()); // signal_id
    tlv_data[10..14].copy_from_slice(&137u32.to_le_bytes()); // chain_id (Polygon)
    payload.extend_from_slice(&tlv_data);

    let payload_size = payload.len() as u32;
    message.extend_from_slice(&payload_size.to_le_bytes()); // payload_size
    message.extend_from_slice(&0u32.to_le_bytes()); // checksum

    message.extend_from_slice(&payload); // actual payload

    message
}

fn main() {
    println!("🧪 Testing Rust Signal Relay Performance");
    println!("{}", "=".repeat(50));

    // Connect multiple consumers
    let consumer_count = 3;
    let mut consumer_threads = Vec::new();

    for i in 0..consumer_count {
        let thread = thread::spawn(move || {
            match UnixStream::connect("/tmp/alphapulse/signals.sock") {
                Ok(mut stream) => {
                    println!("✅ Consumer {} connected", i);
                    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

                    let mut buffer = vec![0u8; 4096];
                    let mut message_count = 0;

                    loop {
                        match stream.read(&mut buffer) {
                            Ok(0) => {
                                println!("Consumer {} disconnected", i);
                                break;
                            }
                            Ok(n) => {
                                message_count += 1;
                                if message_count <= 3 || message_count % 100 == 0 {
                                    println!("📥 Consumer {} received message {}: {} bytes",
                                             i, message_count, n);
                                }

                                if message_count >= 1000 {
                                    break;
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                if message_count > 0 {
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Consumer {} error: {}", i, e);
                                break;
                            }
                        }
                    }

                    println!("👋 Consumer {} received {} messages total", i, message_count);
                }
                Err(e) => {
                    eprintln!("❌ Consumer {} failed to connect: {}", i, e);
                }
            }
        });
        consumer_threads.push(thread);
    }

    // Give consumers time to connect
    thread::sleep(Duration::from_millis(500));

    // Connect as publisher and send messages
    match UnixStream::connect("/tmp/alphapulse/signals.sock") {
        Ok(mut stream) => {
            println!("✅ Publisher connected");

            let message_count = 1000;
            let start = std::time::Instant::now();

            for i in 0..message_count {
                let message = create_test_message(i + 1);
                stream.write_all(&message).expect("Failed to send message");

                if i < 3 || (i + 1) % 100 == 0 {
                    println!("📤 Sent message {}/{}", i + 1, message_count);
                }
            }

            let elapsed = start.elapsed();
            let throughput = message_count as f64 / elapsed.as_secs_f64();

            println!("\n📊 Performance Results:");
            println!("  Messages sent: {}", message_count);
            println!("  Time elapsed: {:?}", elapsed);
            println!("  Throughput: {:.0} messages/second", throughput);

            if throughput > 10000.0 {
                println!("  ✅ Excellent performance (>10k msg/s)");
            } else if throughput > 1000.0 {
                println!("  ✅ Good performance (>1k msg/s)");
            } else {
                println!("  ⚠️  Performance below target (<1k msg/s)");
            }
        }
        Err(e) => {
            eprintln!("❌ Publisher failed to connect: {}", e);
        }
    }

    // Wait for consumers
    for thread in consumer_threads {
        thread.join().ok();
    }

    println!("\n{}", "=".repeat(50));
    println!("✅ Test completed!");
}

// Import the Read trait
use std::io::Read;
