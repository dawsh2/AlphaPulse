// Test accessing the actual event-driven shared memory created by the collector
// This will help us determine if the issue is specific to cross-process access
// of memory initialized by different processes

use alphapulse_common::event_driven_shm::*;
use std::fs::OpenOptions;
use std::sync::atomic::Ordering;
use memmap2::{MmapOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing access to real event-driven shared memory");
    println!("=====================================================");
    
    let path = "./shm/coinbase_trades";
    println!("🔍 Attempting to open: {}", path);
    
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;
    
    let mmap = unsafe { 
        MmapOptions::new()
            .len(file.metadata()?.len() as usize)
            .map_mut(&file)?  // Use map_mut for write access to enable atomic RMW operations
    };
    
    println!("✅ Successfully mapped file");
    println!("   Base address: {:p}", mmap.as_ptr());
    println!("   File size: {} bytes", mmap.len());
    
    let header_ptr = mmap.as_ptr() as *const EventDrivenHeader;
    
    unsafe {
        println!("\n🔍 Reading header information...");
        let header = &*header_ptr;
        
        println!("   Header at: {:p}", header_ptr);
        println!("   version: {}", header.version.load(Ordering::Relaxed));
        println!("   capacity: {}", header.capacity.load(Ordering::Relaxed));
        println!("   write_sequence: {}", header.write_sequence.load(Ordering::Relaxed));
        println!("   writer_pid: {}", header.writer_pid.load(Ordering::Relaxed));
        
        let registry_ptr = &header.reader_registry as *const AtomicReaderRegistry;
        let registry_addr = registry_ptr as usize;
        let header_addr = header_ptr as usize;
        
        println!("\n🔍 Reader registry information...");
        println!("   Registry at: {:p} (offset: {})", registry_ptr, registry_addr - header_addr);
        println!("   Registry alignment: {} bytes", std::mem::align_of::<AtomicReaderRegistry>());
        println!("   Registry 128-byte aligned? {}", registry_addr % 128 == 0);
        
        let active_slots_ptr = &header.reader_registry.active_slots;
        let slots_addr = active_slots_ptr as *const _ as usize;
        
        println!("\n🔍 Active slots information...");
        println!("   active_slots at: {:p}", active_slots_ptr);
        println!("   active_slots alignment: {} bytes", std::mem::align_of_val(active_slots_ptr));
        println!("   active_slots 8-byte aligned? {}", slots_addr % 8 == 0);
        
        println!("\n🔧 Testing atomic load operation...");
        let current_slots = header.reader_registry.active_slots.load(Ordering::SeqCst);
        println!("✅ atomic load successful: current_slots = 0x{:x}", current_slots);
        
        println!("\n🔧 Testing atomic fetch_or operation...");
        println!("   This is where the SIGBUS crash should occur if the issue persists...");
        
        // Test the exact same operation that's failing in the real code
        let test_slot = 0;
        let slot_bit = 1u64 << test_slot;
        
        // Try the fetch_or operation that's causing SIGBUS
        let old_slots = header.reader_registry.active_slots.fetch_or(slot_bit, Ordering::SeqCst);
        println!("✅ atomic fetch_or successful: old_slots = 0x{:x}, new bit = 0x{:x}", old_slots, slot_bit);
        
        // Clean up by clearing the bit we just set
        header.reader_registry.active_slots.fetch_and(!slot_bit, Ordering::SeqCst);
        println!("✅ atomic fetch_and successful (cleanup)");
        
        println!("\n🎉 All atomic operations succeeded!");
        println!("   This suggests the issue is NOT with cross-process access of collector-created memory");
        println!("   The problem might be specific to the async/tokio context or timing");
    }
    
    Ok(())
}