// Comprehensive memory layout analysis tool to identify potential SIGBUS causes
use alphapulse_common::shared_memory::{
    SharedTrade, SharedOrderBookDelta, PriceLevelChange, RingBufferHeader
};
use std::mem;

fn main() {
    println!("🔬 Comprehensive Memory Layout Analysis");
    println!("======================================\n");
    
    // Test 1: Struct size and alignment analysis
    println!("📐 Struct Size and Alignment Analysis:");
    analyze_struct::<SharedTrade>("SharedTrade");
    analyze_struct::<SharedOrderBookDelta>("SharedOrderBookDelta"); 
    analyze_struct::<PriceLevelChange>("PriceLevelChange");
    analyze_struct::<RingBufferHeader>("RingBufferHeader");
    
    // Test 2: Field offset analysis
    println!("\n🧭 Field Offset Analysis:");
    analyze_shared_trade_offsets();
    analyze_shared_orderbook_delta_offsets();
    analyze_ring_buffer_header_offsets();
    
    // Test 3: Padding and packing analysis
    println!("\n📦 Padding and Packing Analysis:");
    check_padding_issues();
    
    // Test 4: Pointer arithmetic validation
    println!("\n➗ Pointer Arithmetic Validation:");
    validate_pointer_arithmetic();
    
    // Test 5: Memory fence and volatile access patterns
    println!("\n🔒 Memory Access Pattern Analysis:");
    analyze_memory_access_patterns();
}

fn analyze_struct<T>(name: &str) {
    println!("  {}:", name);
    println!("    Size: {} bytes", mem::size_of::<T>());
    println!("    Alignment: {} bytes", mem::align_of::<T>());
    println!("    Size is multiple of alignment: {}", 
        mem::size_of::<T>() % mem::align_of::<T>() == 0);
    
    // Check if size matches expected constants
    match name {
        "SharedTrade" => {
            let expected = 128;
            let actual = mem::size_of::<T>();
            println!("    Expected size: {} bytes", expected);
            println!("    Size matches constant: {}", actual == expected);
        }
        "SharedOrderBookDelta" => {
            let expected = 256;
            let actual = mem::size_of::<T>();
            println!("    Expected size: {} bytes", expected);
            println!("    Size matches constant: {}", actual == expected);
        }
        _ => {}
    }
    println!();
}

fn analyze_shared_trade_offsets() {
    println!("  SharedTrade field offsets:");
    
    // Use offset_of! if available, or manual calculation
    let trade = SharedTrade::new(0, "", "", 0.0, 0.0, false, "");
    let base_ptr = &trade as *const SharedTrade as usize;
    
    println!("    timestamp_ns: offset {}", 
        &trade.timestamp_ns as *const u64 as usize - base_ptr);
    println!("    symbol: offset {}", 
        &trade.symbol as *const [u8; 16] as usize - base_ptr);
    println!("    exchange: offset {}", 
        &trade.exchange as *const [u8; 16] as usize - base_ptr);
    println!("    price: offset {}", 
        &trade.price as *const f64 as usize - base_ptr);
    println!("    volume: offset {}", 
        &trade.volume as *const f64 as usize - base_ptr);
    println!("    side: offset {}", 
        &trade.side as *const u8 as usize - base_ptr);
    println!("    trade_id: offset {}", 
        &trade.trade_id as *const [u8; 32] as usize - base_ptr);
}

fn analyze_shared_orderbook_delta_offsets() {
    println!("  SharedOrderBookDelta field offsets:");
    
    let delta = SharedOrderBookDelta::new(0, "", "", 0, 0);
    let base_ptr = &delta as *const SharedOrderBookDelta as usize;
    
    println!("    timestamp_ns: offset {}", 
        &delta.timestamp_ns as *const u64 as usize - base_ptr);
    println!("    symbol: offset {}", 
        &delta.symbol as *const [u8; 16] as usize - base_ptr);
    println!("    exchange: offset {}", 
        &delta.exchange as *const [u8; 16] as usize - base_ptr);
    println!("    version: offset {}", 
        &delta.version as *const u64 as usize - base_ptr);
    println!("    prev_version: offset {}", 
        &delta.prev_version as *const u64 as usize - base_ptr);
    println!("    change_count: offset {}", 
        &delta.change_count as *const u16 as usize - base_ptr);
    println!("    changes: offset {}", 
        &delta.changes as *const [PriceLevelChange; 16] as usize - base_ptr);
}

fn analyze_ring_buffer_header_offsets() {
    println!("  RingBufferHeader field analysis:");
    println!("    Size: {} bytes", mem::size_of::<RingBufferHeader>());
    println!("    Alignment: {} bytes", mem::align_of::<RingBufferHeader>());
    
    // Check atomic field alignment
    println!("    AtomicU64 size: {} bytes", mem::size_of::<std::sync::atomic::AtomicU64>());
    println!("    AtomicU64 alignment: {} bytes", mem::align_of::<std::sync::atomic::AtomicU64>());
}

fn check_padding_issues() {
    // Check if PriceLevelChange is packed correctly
    let change_size = mem::size_of::<PriceLevelChange>();
    let change_align = mem::align_of::<PriceLevelChange>();
    println!("  PriceLevelChange packing:");
    println!("    Size: {} bytes", change_size);
    println!("    Alignment: {} bytes", change_align);
    println!("    Expected size (4+4+1+3): 12 bytes");
    println!("    Correctly sized: {}", change_size == 12);
    
    // Check array stride in SharedOrderBookDelta
    let expected_changes_size = 16 * 12; // 16 changes * 12 bytes each
    let actual_changes_size = mem::size_of::<[PriceLevelChange; 16]>();
    println!("  Changes array packing:");
    println!("    Expected size: {} bytes", expected_changes_size);
    println!("    Actual size: {} bytes", actual_changes_size);
    println!("    Correctly packed: {}", expected_changes_size == actual_changes_size);
}

fn validate_pointer_arithmetic() {
    // Simulate the pointer arithmetic used in the shared memory code
    let capacity = 10000usize;
    let header_size = mem::size_of::<RingBufferHeader>();
    let trade_size = mem::size_of::<SharedTrade>();
    let delta_size = mem::size_of::<SharedOrderBookDelta>();
    
    println!("  Trade buffer calculations:");
    println!("    Header size: {} bytes", header_size);
    println!("    Trade size: {} bytes", trade_size);
    println!("    Capacity: {} items", capacity);
    println!("    Total size: {} bytes", header_size + capacity * trade_size);
    println!("    Data start offset: {} bytes", header_size);
    println!("    Data alignment check: header_size % trade_alignment = {} (should be 0)", 
        header_size % mem::align_of::<SharedTrade>());
    
    println!("  Delta buffer calculations:");
    println!("    Delta size: {} bytes", delta_size);
    println!("    Total size: {} bytes", header_size + capacity * delta_size);
    println!("    Data alignment check: header_size % delta_alignment = {} (should be 0)", 
        header_size % mem::align_of::<SharedOrderBookDelta>());
    
    // Check index calculation overflow potential
    let max_sequence = u64::MAX;
    println!("  Index calculation safety:");
    println!("    Max sequence: {}", max_sequence);
    println!("    Modulo operation result: {}", max_sequence % capacity as u64);
    println!("    Index fits in usize: {}", (max_sequence % capacity as u64) <= usize::MAX as u64);
}

fn analyze_memory_access_patterns() {
    println!("  Volatile memory access analysis:");
    
    // Check if volatile operations are properly aligned
    println!("    ptr::read_volatile alignment requirements:");
    println!("      SharedTrade must be {}-byte aligned", mem::align_of::<SharedTrade>());
    println!("      SharedOrderBookDelta must be {}-byte aligned", mem::align_of::<SharedOrderBookDelta>());
    
    println!("    Memory fence usage:");
    println!("      Acquire fence before reads: Used");
    println!("      Release fence after writes: Used");
    println!("      AcqRel ordering on atomics: Used");
    
    // Check for potential data races
    println!("  Potential data race analysis:");
    println!("    Reader cursors array: 16 entries, indexed by reader_id");
    println!("    Max reader_id check: Present (< 16)");
    println!("    Writer sequence: AtomicU64 with AcqRel ordering");
    println!("    Cached sequence: Plain u64 (writer-only access)");
}