// Debug utility to examine event-driven shared memory layout
use alphapulse_common::event_driven_shm::*;
use alphapulse_common::Result;
use memmap2::Mmap;
use std::fs::OpenOptions;
use std::mem;

fn main() -> Result<()> {
    println!("🔍 Event-Driven Shared Memory Layout Analysis");
    println!("==============================================");
    
    // Print struct sizes
    println!("Struct sizes:");
    println!("  EventDrivenHeader: {} bytes", mem::size_of::<EventDrivenHeader>());
    println!("  AtomicReaderRegistry: {} bytes", mem::size_of::<AtomicReaderRegistry>());
    println!("  EventDrivenReaderCursor: {} bytes", mem::size_of::<EventDrivenReaderCursor>());
    println!("  EventDrivenTrade: {} bytes", mem::size_of::<EventDrivenTrade>());
    
    // Print alignments
    println!("\nStruct alignments:");
    println!("  EventDrivenHeader: {} bytes", mem::align_of::<EventDrivenHeader>());
    println!("  AtomicReaderRegistry: {} bytes", mem::align_of::<AtomicReaderRegistry>());
    println!("  EventDrivenReaderCursor: {} bytes", mem::align_of::<EventDrivenReaderCursor>());
    println!("  EventDrivenTrade: {} bytes", mem::align_of::<EventDrivenTrade>());
    
    // Examine actual file
    let path = "./shm/coinbase_trades";
    println!("\n🔍 Examining file: {}", path);
    
    if let Ok(file) = OpenOptions::new().read(true).open(path) {
        if let Ok(mmap) = unsafe { Mmap::map(&file) } {
            println!("File size: {} bytes", mmap.len());
            println!("Base address: {:p}", mmap.as_ptr());
            
            // Try to read the header
            let header_ptr = mmap.as_ptr() as *const EventDrivenHeader;
            unsafe {
                let header = &*header_ptr;
                println!("\nHeader contents:");
                println!("  version: {}", header.version.load(std::sync::atomic::Ordering::Relaxed));
                println!("  capacity: {}", header.capacity.load(std::sync::atomic::Ordering::Relaxed));
                println!("  write_sequence: {}", header.write_sequence.load(std::sync::atomic::Ordering::Relaxed));
                println!("  writer_pid: {}", header.writer_pid.load(std::sync::atomic::Ordering::Relaxed));
                
                // Check reader registry
                let registry_ptr = &header.reader_registry as *const AtomicReaderRegistry;
                println!("\nReader Registry:");
                println!("  Registry address: {:p} (offset: {})", registry_ptr, registry_ptr as usize - header_ptr as usize);
                println!("  active_slots address: {:p}", &header.reader_registry.active_slots);
                println!("  active_slots value: 0x{:x}", header.reader_registry.active_slots.load(std::sync::atomic::Ordering::Relaxed));
                
                // Check if we can safely read active_slots
                let slots_addr = &header.reader_registry.active_slots as *const _ as usize;
                println!("  active_slots alignment check: addr=0x{:x}, aligned? {}", slots_addr, slots_addr % 8 == 0);
                
                // Try to dump first few bytes of the registry
                let registry_bytes = std::slice::from_raw_parts(registry_ptr as *const u8, 32);
                print!("  Registry first 32 bytes: ");
                for b in registry_bytes {
                    print!("{:02x} ", b);
                }
                println!();
            }
        } else {
            println!("❌ Failed to map file");
        }
    } else {
        println!("❌ Failed to open file");
    }
    
    Ok(())
}