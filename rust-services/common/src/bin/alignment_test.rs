// Test program to specifically trigger alignment issues with PriceLevelChange
use alphapulse_common::shared_memory::{PriceLevelChange, SharedOrderBookDelta};
use std::ptr;

fn main() {
    println!("🚨 Alignment Issue Test for SIGBUS Crash");
    println!("==========================================\n");
    
    // Test 1: Direct struct alignment test
    test_price_level_change_alignment();
    
    // Test 2: Array access alignment test
    test_array_alignment();
    
    // Test 3: Volatile read test (this might crash)
    test_volatile_reads();
    
    // Test 4: Unaligned memory access simulation
    simulate_unaligned_access();
    
    println!("\n✅ All alignment tests completed!");
}

fn test_price_level_change_alignment() {
    println!("🔍 Testing PriceLevelChange direct alignment:");
    
    let mut change = PriceLevelChange::default();
    change.price = 1234.56f32;
    change.volume = 789.01f32;
    change.side_and_action = 0x81;  // high bit set (ask) + action 1
    
    let base_addr = &change as *const PriceLevelChange as usize;
    let price_addr = &change.price as *const f32 as usize;
    let volume_addr = &change.volume as *const f32 as usize;
    
    println!("  Base address: 0x{:x}", base_addr);
    println!("  Price address: 0x{:x} (offset: {})", price_addr, price_addr - base_addr);
    println!("  Volume address: 0x{:x} (offset: {})", volume_addr, volume_addr - base_addr);
    
    println!("  Price alignment (4-byte): {}", price_addr % 4 == 0);
    println!("  Volume alignment (4-byte): {}", volume_addr % 4 == 0);
    
    // Try to access the fields
    println!("  Reading price: {}", change.price);
    println!("  Reading volume: {}", change.volume);
    println!();
}

fn test_array_alignment() {
    println!("🔍 Testing array of PriceLevelChange alignment:");
    
    let changes: [PriceLevelChange; 16] = [PriceLevelChange::default(); 16];
    
    for i in 0..16 {
        let change_addr = &changes[i] as *const PriceLevelChange as usize;
        let price_addr = &changes[i].price as *const f32 as usize;
        let volume_addr = &changes[i].volume as *const f32 as usize;
        
        let price_aligned = price_addr % 4 == 0;
        let volume_aligned = volume_addr % 4 == 0;
        
        if !price_aligned || !volume_aligned {
            println!("  ❌ Index {}: Misaligned! Change @ 0x{:x}, Price aligned: {}, Volume aligned: {}", 
                i, change_addr, price_aligned, volume_aligned);
        } else if i < 3 || i == 15 {
            println!("  ✅ Index {}: Aligned. Change @ 0x{:x}", i, change_addr);
        }
    }
    println!();
}

fn test_volatile_reads() {
    println!("🔍 Testing volatile reads on SharedOrderBookDelta:");
    
    let mut delta = SharedOrderBookDelta::new(
        1234567890123u64,
        "BTC-USD",
        "coinbase",
        1001,
        1000
    );
    
    // Add some changes to the delta
    for i in 0..5 {
        let price = 50000.0f32 + i as f32;
        let volume = 0.5f32 + (i as f32 * 0.1);
        delta.add_change(price as f64, volume as f64, i % 2 == 1, 1);
    }
    
    println!("  Delta created with {} changes", delta.change_count);
    println!("  Base address: {:p}", &delta);
    println!("  Changes array address: {:p}", &delta.changes);
    
    // Try volatile read of the whole struct (this might cause SIGBUS)
    println!("  Attempting volatile read of entire delta structure...");
    let delta_ptr = &delta as *const SharedOrderBookDelta;
    let read_delta = unsafe { ptr::read_volatile(delta_ptr) };
    
    println!("  ✅ Volatile read successful! Read {} changes", read_delta.change_count);
    
    // Try accessing individual changes
    for i in 0..read_delta.change_count as usize {
        let change = &read_delta.changes[i];
        println!("    Change {}: price={}, volume={}, side_action=0x{:02x}", 
            i, change.price, change.volume, change.side_and_action);
    }
    println!();
}

fn simulate_unaligned_access() {
    println!("🔍 Simulating unaligned memory access:");
    
    // Create a byte buffer with intentionally unaligned f32 data
    let mut buffer = vec![0u8; 100];
    
    // Write f32 values at unaligned positions
    let test_value = 1234.5678f32;
    let test_bytes = test_value.to_le_bytes();
    
    for offset in 1..=5 {
        // Write f32 at unaligned offset
        buffer[offset..offset+4].copy_from_slice(&test_bytes);
        
        let ptr = buffer.as_ptr().wrapping_add(offset) as *const f32;
        let addr = ptr as usize;
        
        println!("  Offset {}: address 0x{:x}, aligned: {}", 
            offset, addr, addr % 4 == 0);
        
        // Try to read the f32 (this might cause SIGBUS on strict platforms)
        if addr % 4 == 0 {
            let value = unsafe { ptr::read(ptr) };
            println!("    ✅ Read value: {}", value);
        } else {
            println!("    ⚠️  Unaligned - would cause SIGBUS on strict platforms");
            // Use unaligned read instead
            let value = unsafe { ptr::read_unaligned(ptr) };
            println!("    ✅ Unaligned read value: {}", value);
        }
    }
    println!();
}

// Default impl is already in shared_memory.rs