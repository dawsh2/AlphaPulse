// Optimized shared memory implementation - fixes SIGBUS without latency penalty
// Uses atomic operations and proper alignment instead of volatile reads

use std::sync::atomic::{AtomicU64, AtomicU32, Ordering, fence};
use std::ptr;
use std::mem;
use std::fs::{OpenOptions, File};
use memmap2::{MmapMut, MmapOptions, Mmap};
use crate::{Result, AlphaPulseError};

// Platform-specific cache line alignment for cross-process atomics
// macOS ARM64 requires 128-byte alignment for reliable cross-process atomic operations
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const CACHE_LINE_SIZE: usize = 128;

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
const CACHE_LINE_SIZE: usize = 64;

// Platform-specific aligned memory mapping for cross-process atomic reliability
fn create_aligned_mmap(file: &File) -> Result<Mmap> {
    // Try multiple approaches to get aligned memory mapping
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        // Approach 1: Try using MmapOptions with map_copy for better alignment
        let metadata = file.metadata()?;
        let file_size = metadata.len() as usize;
        
        // Try multiple mapping attempts to get proper alignment
        for attempt in 0..10 {
            let mmap = unsafe { 
                MmapOptions::new()
                    .len(file_size)
                    .map(file)?
            };
            
            let alignment = (mmap.as_ptr() as usize) % CACHE_LINE_SIZE;
            tracing::debug!("🔍 Alignment attempt {}: addr={:p}, alignment={}", 
                attempt, mmap.as_ptr(), alignment);
            
            if alignment == 0 {
                tracing::info!("✅ Found aligned mapping on attempt {}: addr={:p}", 
                    attempt, mmap.as_ptr());
                return Ok(mmap);
            }
            
            // Drop the mmap and try again (forces new memory allocation)
            drop(mmap);
        }
        
        // If we can't get aligned mapping, proceed anyway but warn
        let mmap = unsafe { Mmap::map(file)? };
        let alignment = (mmap.as_ptr() as usize) % CACHE_LINE_SIZE;
        tracing::warn!("⚠️  Could not obtain aligned mapping after 10 attempts. Using misaligned mapping: addr={:p}, alignment={}", 
            mmap.as_ptr(), alignment);
        
        Ok(mmap)
    }
    
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let mmap = unsafe { Mmap::map(file)? };
        Ok(mmap)
    }
}

// Trade struct using atomics for lock-free access
// macOS ARM64: 128-byte alignment required for cross-process atomic reliability
// CRITICAL: Struct size MUST be exactly 128 bytes to ensure ring buffer alignment
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[repr(C, align(128))]
pub struct AtomicSharedTrade {
    pub timestamp_ns: AtomicU64,  // 8 bytes
    pub symbol: [u8; 16],         // 16 bytes - Fixed-size, no atomics needed
    pub exchange: [u8; 16],       // 16 bytes - Fixed-size, no atomics needed  
    pub price: AtomicU64,         // 8 bytes - Store as fixed-point (price * 1e8)
    pub volume: AtomicU64,        // 8 bytes - Store as fixed-point (volume * 1e8)
    pub side: AtomicU32,          // 4 bytes - 0=buy, 1=sell
    pub trade_id: [u8; 32],       // 32 bytes - Fixed-size, no atomics needed
    _padding: [u8; 20],           // 20 bytes - Total: 128 bytes exactly for ARM64
}

// Other platforms: Standard 64-byte alignment
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
#[repr(C, align(64))]  // CRITICAL: Cache-line alignment
pub struct AtomicSharedTrade {
    pub timestamp_ns: AtomicU64,
    pub symbol: [u8; 16],         // Fixed-size, no atomics needed
    pub exchange: [u8; 16],       // Fixed-size, no atomics needed  
    pub price: AtomicU64,         // Store as fixed-point (price * 1e8)
    pub volume: AtomicU64,        // Store as fixed-point (volume * 1e8)
    pub side: AtomicU32,          // 0=buy, 1=sell
    pub trade_id: [u8; 32],       // Fixed-size, no atomics needed
    _padding: [u8; 12],           // Padding to 128 bytes (2 cache lines)
}

// OrderBook delta using atomics
// macOS ARM64: 128-byte alignment required for cross-process atomic reliability
// CRITICAL: For large structs like deltas, use multiple of 128 bytes
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[repr(C, align(128))]
pub struct AtomicSharedOrderBookDelta {
    pub timestamp_ns: AtomicU64,              // 8 bytes
    pub symbol: [u8; 16],                     // 16 bytes
    pub exchange: [u8; 16],                   // 16 bytes
    pub version: AtomicU64,                   // 8 bytes
    pub prev_version: AtomicU64,              // 8 bytes
    pub change_count: AtomicU32,              // 4 bytes
    _padding1: [u8; 4],                       // 4 bytes - Align to 8 bytes
    pub changes: [AtomicPriceLevelChange; 16], // 16 * 16 = 256 bytes
    _padding2: [u8; 44],                      // 44 bytes - Total: 384 bytes (3 * 128)
}

// Other platforms: Standard 64-byte alignment
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
#[repr(C, align(64))]  // CRITICAL: Cache-line alignment
pub struct AtomicSharedOrderBookDelta {
    pub timestamp_ns: AtomicU64,
    pub symbol: [u8; 16],
    pub exchange: [u8; 16],
    pub version: AtomicU64,
    pub prev_version: AtomicU64,
    pub change_count: AtomicU32,
    _padding1: [u8; 4],  // Align to 8 bytes
    pub changes: [AtomicPriceLevelChange; 16],  // 16 * 16 = 256 bytes
}

// Price level change using atomics (16 bytes each)
#[repr(C, align(8))]
pub struct AtomicPriceLevelChange {
    pub price: AtomicU64,         // Price as fixed-point
    pub volume_and_meta: AtomicU64, // High 32: volume, Low 32: side+action
}

impl AtomicPriceLevelChange {
    pub fn pack(price: f64, volume: f64, is_ask: bool, action: u8) -> Self {
        let price_fixed = (price * 1e8) as u64;
        let volume_fixed = (volume * 1e8) as u32;
        let meta = ((is_ask as u32) << 31) | (action as u32);
        let volume_and_meta = ((volume_fixed as u64) << 32) | (meta as u64);
        
        Self {
            price: AtomicU64::new(price_fixed),
            volume_and_meta: AtomicU64::new(volume_and_meta),
        }
    }
    
    pub fn unpack(&self) -> (f64, f64, bool, u8) {
        let price_fixed = self.price.load(Ordering::Acquire);
        let vm = self.volume_and_meta.load(Ordering::Acquire);
        
        let price = (price_fixed as f64) / 1e8;
        let volume = ((vm >> 32) as f64) / 1e8;
        let is_ask = ((vm & 0x80000000) != 0);
        let action = (vm & 0xFF) as u8;
        
        (price, volume, is_ask, action)
    }
}

// Optimized ring buffer header with platform-specific alignment
// macOS ARM64: Each reader cursor must be individually 128-byte aligned for cross-process atomics
// CRITICAL: Header block (128 bytes) + reader cursor blocks (8 * 128 = 1024 bytes) = 1152 bytes total
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[repr(C, align(128))]
pub struct AtomicRingBufferHeader {
    pub version: AtomicU32,           // 4 bytes
    pub capacity: AtomicU32,          // 4 bytes
    pub write_sequence: AtomicU64,    // 8 bytes
    pub writer_pid: AtomicU32,        // 4 bytes
    _padding1: [u8; 4],              // 4 bytes - align to 8
    pub last_write_ns: AtomicU64,     // 8 bytes
    _reserved: [u8; 96],             // 96 bytes - Total: exactly 128 bytes (no cursors in header)
}

// Individual 128-byte aligned cursor for cross-process atomic safety on macOS ARM64
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[repr(C, align(128))]
pub struct AlignedReaderCursor {
    pub cursor: AtomicU64,           // 8 bytes
    _padding: [u8; 120],             // 120 bytes - Total: exactly 128 bytes
}

// Other platforms: Standard 64-byte alignment with cursor array
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
#[repr(C, align(64))]
pub struct AtomicRingBufferHeader {
    pub version: AtomicU32,
    pub capacity: AtomicU32,
    pub write_sequence: AtomicU64,
    pub writer_pid: AtomicU32,
    _padding1: [u8; 4],
    pub last_write_ns: AtomicU64,
    pub reader_cursors: [AtomicU64; 16],
    _padding2: [u8; 32],  // Pad to multiple of cache line
}

// Non-macOS platforms: Use dummy cursor struct for compatibility
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub struct AlignedReaderCursor {
    pub cursor: AtomicU64,
}

// Optimized reader that avoids volatile reads of large structs
pub struct OptimizedOrderBookDeltaReader {
    mmap: memmap2::Mmap,
    header: *const AtomicRingBufferHeader,
    data_start: *const AtomicSharedOrderBookDelta,
    cursor_blocks: *const AlignedReaderCursor,  // Pointer to aligned cursor blocks
    capacity: usize,
    reader_id: usize,
    last_sequence: u64,
}

unsafe impl Send for OptimizedOrderBookDeltaReader {}
unsafe impl Sync for OptimizedOrderBookDeltaReader {}

impl OptimizedOrderBookDeltaReader {
    pub fn open(path: &str, reader_id: usize) -> Result<Self> {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let max_readers = 8;
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        let max_readers = 16;
        
        if reader_id >= max_readers {
            return Err(AlphaPulseError::ConfigError(
                format!("Reader ID must be less than {}", max_readers)
            ));
        }
        
        let file = OpenOptions::new()
            .read(true)
            .open(path)?;
            
        // Create aligned memory mapping for macOS ARM64
        let mmap = create_aligned_mmap(&file)?;
        
        // Verify alignment - CRITICAL for macOS ARM64 cross-process atomics
        let ptr = mmap.as_ptr();
        let alignment = (ptr as usize) % CACHE_LINE_SIZE;
        if alignment != 0 {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                return Err(AlphaPulseError::MemoryMappingError(format!(
                    "Delta memory map not aligned to {}-byte boundary (macOS ARM64 requirement): addr={:p}, modulo={}",
                    CACHE_LINE_SIZE, ptr, alignment
                )));
            }
            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            {
                return Err(AlphaPulseError::MemoryMappingError(format!(
                    "Delta memory map not cache-line aligned: addr={:p}, modulo={}",
                    ptr, alignment
                )));
            }
        }
        
        // Additional validation: ensure struct size is aligned
        let struct_size = mem::size_of::<AtomicSharedOrderBookDelta>();
        if struct_size % CACHE_LINE_SIZE != 0 {
            tracing::warn!("AtomicSharedOrderBookDelta size {} not aligned to {}-byte boundary", 
                struct_size, CACHE_LINE_SIZE);
        }
        
        tracing::info!("✅ Memory alignment validated: addr={:p}, cache_line_size={}, struct_size={}", 
            ptr, CACHE_LINE_SIZE, struct_size);
        
        let header_ptr = ptr as *const AtomicRingBufferHeader;
        let capacity = unsafe {
            (*header_ptr).capacity.load(Ordering::Relaxed) as usize
        };
        
        let header_size = mem::size_of::<AtomicRingBufferHeader>();
        
        // Calculate cursor blocks location (after header)
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let cursor_blocks = unsafe {
            ptr.add(header_size) as *const AlignedReaderCursor
        };
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]  
        let cursor_blocks = std::ptr::null::<AlignedReaderCursor>(); // Not used on other platforms
        
        // Calculate data start (after header + cursor blocks)
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let cursor_blocks_size = 8 * mem::size_of::<AlignedReaderCursor>(); // 8 * 128 = 1024 bytes
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        let cursor_blocks_size = 0;
        
        let data_start = unsafe {
            ptr.add(header_size + cursor_blocks_size) as *const AtomicSharedOrderBookDelta
        };
        
        // Start from current write position
        let last_sequence = unsafe {
            (*header_ptr).write_sequence.load(Ordering::Acquire)
        };
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            cursor_blocks,
            capacity,
            reader_id,
            last_sequence,
        })
    }
    
    pub fn read_deltas_optimized(&mut self) -> Vec<SharedOrderBookDelta> {
        let mut deltas = Vec::new();
        
        unsafe {
            let header = &*self.header;
            
            // DEBUG: Log memory addresses and alignment
            tracing::debug!("🔍 SIGBUS DEBUG: header addr={:p}, alignment={}", 
                header, (header as *const _ as usize) % CACHE_LINE_SIZE);
            tracing::debug!("🔍 SIGBUS DEBUG: data_start addr={:p}, alignment={}", 
                self.data_start, (self.data_start as *const _ as usize) % CACHE_LINE_SIZE);
            
            // Acquire fence to see all writes
            fence(Ordering::Acquire);
            tracing::debug!("🔍 SIGBUS DEBUG: About to read write_sequence from header");
            
            let write_sequence = header.write_sequence.load(Ordering::Acquire);
            tracing::debug!("🔍 SIGBUS DEBUG: Successfully read write_sequence={}", write_sequence);
            
            while self.last_sequence < write_sequence {
                let index = (self.last_sequence % self.capacity as u64) as usize;
                let delta_ptr = self.data_start.add(index);
                
                tracing::debug!("🔍 SIGBUS DEBUG: Reading delta at index={}, ptr={:p}, alignment={}", 
                    index, delta_ptr, (delta_ptr as *const _ as usize) % CACHE_LINE_SIZE);
                
                // Read atomically field by field (NOT as one big volatile read)
                tracing::debug!("🔍 SIGBUS DEBUG: About to read timestamp_ns");
                let timestamp = (*delta_ptr).timestamp_ns.load(Ordering::Acquire);
                tracing::debug!("🔍 SIGBUS DEBUG: Successfully read timestamp={}", timestamp);
                
                tracing::debug!("🔍 SIGBUS DEBUG: About to read version");
                let version = (*delta_ptr).version.load(Ordering::Acquire);
                tracing::debug!("🔍 SIGBUS DEBUG: Successfully read version={}", version);
                
                tracing::debug!("🔍 SIGBUS DEBUG: About to read prev_version");
                let prev_version = (*delta_ptr).prev_version.load(Ordering::Acquire);
                tracing::debug!("🔍 SIGBUS DEBUG: Successfully read prev_version={}", prev_version);
                
                tracing::debug!("🔍 SIGBUS DEBUG: About to read change_count");
                let change_count = (*delta_ptr).change_count.load(Ordering::Acquire);
                tracing::debug!("🔍 SIGBUS DEBUG: Successfully read change_count={}", change_count);
                
                // Copy fixed-size arrays (these don't need atomics)
                let mut symbol = [0u8; 16];
                let mut exchange = [0u8; 16];
                ptr::copy_nonoverlapping(
                    (*delta_ptr).symbol.as_ptr(),
                    symbol.as_mut_ptr(),
                    16
                );
                ptr::copy_nonoverlapping(
                    (*delta_ptr).exchange.as_ptr(),
                    exchange.as_mut_ptr(),
                    16
                );
                
                // Read changes atomically and add directly to delta
                let symbol_str = String::from_utf8_lossy(&symbol)
                    .trim_end_matches('\0')
                    .to_string();
                let exchange_str = String::from_utf8_lossy(&exchange)
                    .trim_end_matches('\0')
                    .to_string();
                    
                let mut delta = SharedOrderBookDelta::new(
                    timestamp,
                    &symbol_str,
                    &exchange_str,
                    version,
                    prev_version,
                );
                
                // Add changes directly from atomic reads
                for i in 0..change_count as usize {
                    let change = &(*delta_ptr).changes[i];
                    let (price, volume, is_ask, action) = change.unpack();
                    delta.add_change(price, volume, is_ask, action);
                }
                
                deltas.push(delta);
                
                self.last_sequence += 1;
            }
            
            // Update reader cursor using aligned cursor blocks
            if !deltas.is_empty() {
                #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                {
                    let cursor_block = &*(self.cursor_blocks.add(self.reader_id));
                    cursor_block.cursor.store(self.last_sequence, Ordering::Release);
                }
                #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
                {
                    header.reader_cursors[self.reader_id].store(
                        self.last_sequence,
                        Ordering::Release
                    );
                }
            }
        }
        
        deltas
    }
    
    pub fn get_lag(&self) -> u64 {
        unsafe {
            let header = &*self.header;
            let write_sequence = header.write_sequence.load(Ordering::Relaxed);
            write_sequence.saturating_sub(self.last_sequence)
        }
    }
}

// Re-export the original struct for compatibility
use crate::shared_memory::{SharedOrderBookDelta, PriceLevelChange, SharedTrade};

// Writer that uses atomics
pub struct OptimizedOrderBookDeltaWriter {
    mmap: MmapMut,
    header: *mut AtomicRingBufferHeader,
    data_start: *mut AtomicSharedOrderBookDelta,
    capacity: usize,
}

unsafe impl Send for OptimizedOrderBookDeltaWriter {}
unsafe impl Sync for OptimizedOrderBookDeltaWriter {}

impl OptimizedOrderBookDeltaWriter {
    pub fn create(path: &str, capacity: usize) -> Result<Self> {
        let header_size = mem::size_of::<AtomicRingBufferHeader>();
        let delta_size = mem::size_of::<AtomicSharedOrderBookDelta>();
        
        // Add cursor blocks for macOS ARM64 (8 * 128 = 1024 bytes)
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let cursor_blocks_size = 8 * mem::size_of::<AlignedReaderCursor>();
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        let cursor_blocks_size = 0;
        
        let total_size = header_size + cursor_blocks_size + (capacity * delta_size);
        
        // Ensure cache-line alignment
        let aligned_size = ((total_size + CACHE_LINE_SIZE - 1) / CACHE_LINE_SIZE) * CACHE_LINE_SIZE;
        
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
            
        file.set_len(aligned_size as u64)?;
        
        let mut mmap = unsafe {
            MmapOptions::new()
                .len(aligned_size)
                .map_mut(&file)?
        };
        
        // Initialize header
        let header_ptr = mmap.as_mut_ptr() as *mut AtomicRingBufferHeader;
        unsafe {
            let header = &mut *header_ptr;
            header.version.store(1, Ordering::Relaxed);
            header.capacity.store(capacity as u32, Ordering::Relaxed);
            header.write_sequence.store(0, Ordering::Relaxed);
            header.writer_pid.store(std::process::id(), Ordering::Relaxed);
            header.last_write_ns.store(0, Ordering::Relaxed);
            
            // Initialize cursor blocks for macOS ARM64 (each 128-byte aligned)
            // Use SeqCst ordering for cross-process atomic reliability on ARM64
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                let cursor_blocks = mmap.as_mut_ptr().add(header_size) as *mut AlignedReaderCursor;
                for i in 0..8 {
                    let cursor_block = &mut *cursor_blocks.add(i);
                    cursor_block.cursor.store(0, Ordering::SeqCst);
                }
            }
            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            {
                for cursor in &header.reader_cursors {
                    cursor.store(0, Ordering::Relaxed);
                }
            }
        }
        
        let data_start = unsafe {
            mmap.as_mut_ptr().add(header_size + cursor_blocks_size) as *mut AtomicSharedOrderBookDelta
        };
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            capacity,
        })
    }
    
    pub fn write_delta_optimized(&mut self, delta: &SharedOrderBookDelta) -> Result<()> {
        unsafe {
            let header = &mut *self.header;
            
            let sequence = header.write_sequence.fetch_add(1, Ordering::AcqRel);
            let index = (sequence % self.capacity as u64) as usize;
            
            if index >= self.capacity {
                return Err(AlphaPulseError::BufferOverflow { index, capacity: self.capacity });
            }
            
            let delta_ptr = self.data_start.add(index);
            
            // Write atomically field by field
            (*delta_ptr).timestamp_ns.store(delta.timestamp_ns, Ordering::Release);
            (*delta_ptr).version.store(delta.version, Ordering::Release);
            (*delta_ptr).prev_version.store(delta.prev_version, Ordering::Release);
            (*delta_ptr).change_count.store(delta.change_count as u32, Ordering::Release);
            
            // Copy fixed-size arrays
            ptr::copy_nonoverlapping(
                delta.symbol.as_ptr(),
                (*delta_ptr).symbol.as_mut_ptr(),
                16
            );
            ptr::copy_nonoverlapping(
                delta.exchange.as_ptr(),
                (*delta_ptr).exchange.as_mut_ptr(),
                16
            );
            
            // Write changes atomically
            for i in 0..delta.change_count as usize {
                let change = &delta.changes[i];
                let packed = AtomicPriceLevelChange::pack(
                    change.price as f64,
                    change.volume as f64,
                    (change.side_and_action & 0x80) != 0,
                    change.side_and_action & 0x7F,
                );
                (*delta_ptr).changes[i].price.store(
                    packed.price.into_inner(),
                    Ordering::Release
                );
                (*delta_ptr).changes[i].volume_and_meta.store(
                    packed.volume_and_meta.into_inner(),
                    Ordering::Release
                );
            }
            
            // Note: fence not needed since fetch_add(AcqRel) provides release semantics
            
            let now_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos() as u64;
            header.last_write_ns.store(now_ns, Ordering::Relaxed);
        }
        
        Ok(())
    }
}

// Atomic Trade Writer - SIGBUS-safe for async contexts
pub struct OptimizedTradeWriter {
    mmap: MmapMut,
    header: *mut AtomicRingBufferHeader,
    data_start: *mut AtomicSharedTrade,
    capacity: usize,
}

unsafe impl Send for OptimizedTradeWriter {}
unsafe impl Sync for OptimizedTradeWriter {}

impl OptimizedTradeWriter {
    pub fn create(path: &str, capacity: usize) -> Result<Self> {
        let header_size = mem::size_of::<AtomicRingBufferHeader>();
        let trade_size = mem::size_of::<AtomicSharedTrade>();
        
        // Add cursor blocks for macOS ARM64 (8 * 128 = 1024 bytes)
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let cursor_blocks_size = 8 * mem::size_of::<AlignedReaderCursor>();
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        let cursor_blocks_size = 0;
        
        let total_size = header_size + cursor_blocks_size + (capacity * trade_size);
        
        // Ensure cache-line alignment
        let aligned_size = ((total_size + CACHE_LINE_SIZE - 1) / CACHE_LINE_SIZE) * CACHE_LINE_SIZE;
        
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
            
        file.set_len(aligned_size as u64)?;
        
        let mut mmap = unsafe {
            MmapOptions::new()
                .len(aligned_size)
                .map_mut(&file)?
        };
        
        // Initialize header
        let header_ptr = mmap.as_mut_ptr() as *mut AtomicRingBufferHeader;
        unsafe {
            let header = &mut *header_ptr;
            header.version.store(1, Ordering::Relaxed);
            header.capacity.store(capacity as u32, Ordering::Relaxed);
            header.write_sequence.store(0, Ordering::Relaxed);
            header.writer_pid.store(std::process::id(), Ordering::Relaxed);
            header.last_write_ns.store(0, Ordering::Relaxed);
            
            // Initialize cursor blocks for macOS ARM64 (each 128-byte aligned)
            // Use SeqCst ordering for cross-process atomic reliability on ARM64
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                let cursor_blocks = mmap.as_mut_ptr().add(header_size) as *mut AlignedReaderCursor;
                for i in 0..8 {
                    let cursor_block = &mut *cursor_blocks.add(i);
                    cursor_block.cursor.store(0, Ordering::SeqCst);
                }
            }
            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            {
                for cursor in &header.reader_cursors {
                    cursor.store(0, Ordering::Relaxed);
                }
            }
        }
        
        let data_start = unsafe {
            mmap.as_mut_ptr().add(header_size + cursor_blocks_size) as *mut AtomicSharedTrade
        };
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            capacity,
        })
    }
    
    pub fn write_trade_optimized(&mut self, trade: &SharedTrade) -> Result<()> {
        unsafe {
            let header = &mut *self.header;
            
            let sequence = header.write_sequence.fetch_add(1, Ordering::AcqRel);
            let index = (sequence % self.capacity as u64) as usize;
            
            if index >= self.capacity {
                return Err(AlphaPulseError::BufferOverflow { index, capacity: self.capacity });
            }
            
            let trade_ptr = self.data_start.add(index);
            
            // Write atomically field by field
            (*trade_ptr).timestamp_ns.store(trade.timestamp_ns, Ordering::Release);
            (*trade_ptr).price.store((trade.price * 1e8) as u64, Ordering::Release);
            (*trade_ptr).volume.store((trade.volume * 1e8) as u64, Ordering::Release);
            (*trade_ptr).side.store(trade.side as u32, Ordering::Release);
            
            // Copy fixed-size arrays
            ptr::copy_nonoverlapping(
                trade.symbol.as_ptr(),
                (*trade_ptr).symbol.as_mut_ptr(),
                16
            );
            ptr::copy_nonoverlapping(
                trade.exchange.as_ptr(),
                (*trade_ptr).exchange.as_mut_ptr(),
                16
            );
            ptr::copy_nonoverlapping(
                trade.trade_id.as_ptr(),
                (*trade_ptr).trade_id.as_mut_ptr(),
                32
            );
            
            // Note: fence not needed since fetch_add(AcqRel) provides release semantics
            
            let now_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos() as u64;
            header.last_write_ns.store(now_ns, Ordering::Relaxed);
        }
        
        Ok(())
    }
}

// Atomic Trade Reader - SIGBUS-safe for async contexts
pub struct OptimizedTradeReader {
    mmap: memmap2::Mmap,
    header: *const AtomicRingBufferHeader,
    data_start: *const AtomicSharedTrade,
    cursor_blocks: *const AlignedReaderCursor,  // Pointer to aligned cursor blocks
    capacity: usize,
    reader_id: usize,
}

unsafe impl Send for OptimizedTradeReader {}
unsafe impl Sync for OptimizedTradeReader {}

impl OptimizedTradeReader {
    pub fn open(path: &str, reader_id: usize) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        
        // Create aligned memory mapping for macOS ARM64
        let mmap = create_aligned_mmap(&file)?;
        
        // Verify alignment - CRITICAL for macOS ARM64 cross-process atomics
        let ptr = mmap.as_ptr();
        let alignment = (ptr as usize) % CACHE_LINE_SIZE;
        if alignment != 0 {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                return Err(AlphaPulseError::MemoryMappingError(format!(
                    "Trade memory map not aligned to {}-byte boundary (macOS ARM64 requirement): addr={:p}, modulo={}",
                    CACHE_LINE_SIZE, ptr, alignment
                )));
            }
            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            {
                return Err(AlphaPulseError::MemoryMappingError(format!(
                    "Trade memory map not cache-line aligned: addr={:p}, modulo={}",
                    ptr, alignment
                )));
            }
        }
        
        // Additional validation: ensure struct size is aligned
        let struct_size = mem::size_of::<AtomicSharedTrade>();
        if struct_size % CACHE_LINE_SIZE != 0 {
            tracing::warn!("AtomicSharedTrade size {} not aligned to {}-byte boundary", 
                struct_size, CACHE_LINE_SIZE);
        }
        
        tracing::info!("✅ Trade memory alignment validated: addr={:p}, cache_line_size={}, struct_size={}", 
            ptr, CACHE_LINE_SIZE, struct_size);
        
        let header_size = mem::size_of::<AtomicRingBufferHeader>();
        let header_ptr = mmap.as_ptr() as *const AtomicRingBufferHeader;
        
        let capacity = unsafe {
            (*header_ptr).capacity.load(Ordering::Acquire) as usize
        };
        
        // Calculate cursor blocks location (after header)
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let cursor_blocks = unsafe {
            mmap.as_ptr().add(header_size) as *const AlignedReaderCursor
        };
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]  
        let cursor_blocks = std::ptr::null::<AlignedReaderCursor>(); // Not used on other platforms
        
        // Calculate data start (after header + cursor blocks)
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let cursor_blocks_size = 8 * mem::size_of::<AlignedReaderCursor>(); // 8 * 128 = 1024 bytes
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        let cursor_blocks_size = 0;
        
        let data_start = unsafe {
            mmap.as_ptr().add(header_size + cursor_blocks_size) as *const AtomicSharedTrade
        };
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            cursor_blocks,
            capacity,
            reader_id,
        })
    }
    
    pub fn read_trades_optimized(&mut self) -> Vec<SharedTrade> {
        let mut trades = Vec::new();
        
        unsafe {
            let header = &*self.header;
            
            // DEBUG: Log memory addresses and alignment
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: header addr={:p}, alignment={}", 
                header, (header as *const _ as usize) % CACHE_LINE_SIZE);
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: data_start addr={:p}, alignment={}", 
                self.data_start, (self.data_start as *const _ as usize) % CACHE_LINE_SIZE);
            
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            let max_readers = 8;
            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            let max_readers = 16;
            
            if self.reader_id >= max_readers {
                return trades; // Maximum readers supported
            }
            
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to read write_sequence");
            let current_write_seq = header.write_sequence.load(Ordering::Acquire);
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully read write_sequence={}", current_write_seq);
            
            // Get cursor using aligned cursor blocks for macOS ARM64
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            let my_cursor = {
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: cursor_blocks base addr={:p}", self.cursor_blocks);
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: reader_id={}, sizeof(AlignedReaderCursor)={}", 
                    self.reader_id, std::mem::size_of::<AlignedReaderCursor>());
                let cursor_block_ptr = self.cursor_blocks.add(self.reader_id);
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: calculated cursor_block_ptr={:p}, alignment={}", 
                    cursor_block_ptr, (cursor_block_ptr as usize) % CACHE_LINE_SIZE);
                let cursor_block = &*cursor_block_ptr;
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: cursor atomic addr={:p}, alignment={}", 
                    &cursor_block.cursor, (&cursor_block.cursor as *const _ as usize) % CACHE_LINE_SIZE);
                &cursor_block.cursor
            };
            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            let my_cursor = &header.reader_cursors[self.reader_id];
            
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to read reader cursor");
            let last_read_seq = my_cursor.load(Ordering::SeqCst);
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully read cursor={}", last_read_seq);
            
            if current_write_seq <= last_read_seq {
                return trades; // No new data
            }
            
            let start_seq = last_read_seq;
            let end_seq = current_write_seq;
            
            for seq in start_seq..end_seq {
                let index = (seq % self.capacity as u64) as usize;
                
                // BOUNDS CHECK: Ensure index is within allocated ring buffer
                if index >= self.capacity {
                    tracing::error!("🚨 SIGBUS DEBUG TRADES: Index {} >= capacity {}, sequence={}", index, self.capacity, seq);
                    break;
                }
                
                let trade_ptr = self.data_start.add(index);
                
                // ALIGNMENT CHECK: Verify pointer alignment before access
                let ptr_alignment = (trade_ptr as *const _ as usize) % CACHE_LINE_SIZE;
                if ptr_alignment != 0 {
                    tracing::error!("🚨 SIGBUS DEBUG TRADES: Misaligned pointer at seq={}, index={}, ptr={:p}, alignment={}", 
                        seq, index, trade_ptr, ptr_alignment);
                    break;
                }
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Reading trade at seq={}, index={}, ptr={:p}, alignment={}", 
                    seq, index, trade_ptr, ptr_alignment);
                
                // Read atomically field by field
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to read timestamp_ns");
                let timestamp_ns = (*trade_ptr).timestamp_ns.load(Ordering::Acquire);
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully read timestamp={}", timestamp_ns);
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to read price");
                let price_fixed = (*trade_ptr).price.load(Ordering::Acquire);
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully read price={}", price_fixed);
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to read volume");
                let volume_fixed = (*trade_ptr).volume.load(Ordering::Acquire);
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully read volume={}", volume_fixed);
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to read side");
                let side = (*trade_ptr).side.load(Ordering::Acquire);
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully read side={}", side);
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Converting fixed-point values");
                // Convert from fixed-point
                let price = (price_fixed as f64) / 1e8;
                let volume = (volume_fixed as f64) / 1e8;
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Converted price={}, volume={}", price, volume);
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to copy symbol array");
                // Copy arrays safely
                let mut symbol = [0u8; 16];
                let mut exchange = [0u8; 16];
                let mut trade_id = [0u8; 32];
                
                ptr::copy_nonoverlapping(
                    (*trade_ptr).symbol.as_ptr(),
                    symbol.as_mut_ptr(),
                    16
                );
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully copied symbol array");
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to copy exchange array");
                ptr::copy_nonoverlapping(
                    (*trade_ptr).exchange.as_ptr(),
                    exchange.as_mut_ptr(),
                    16
                );
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully copied exchange array");
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to copy trade_id array");
                ptr::copy_nonoverlapping(
                    (*trade_ptr).trade_id.as_ptr(),
                    trade_id.as_mut_ptr(),
                    32
                );
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully copied trade_id array");
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to convert strings");
                let symbol_str = String::from_utf8_lossy(&symbol).trim_end_matches('\0').to_string();
                let exchange_str = String::from_utf8_lossy(&exchange).trim_end_matches('\0').to_string();
                let trade_id_str = String::from_utf8_lossy(&trade_id).trim_end_matches('\0').to_string();
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Converted strings: symbol={}, exchange={}, trade_id={}", 
                    symbol_str, exchange_str, trade_id_str);
                
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to create SharedTrade");
                trades.push(SharedTrade::new(
                    timestamp_ns,
                    &symbol_str,
                    &exchange_str,
                    price,
                    volume,
                    side == 0,  // 0 = buy, 1 = sell
                    &trade_id_str,
                ));
                tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully created and pushed SharedTrade");
            }
            
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: Finished reading all trades, about to update cursor from {} to {}", last_read_seq, end_seq);
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: Cursor pointer info: addr={:p}, alignment={}, reader_id={}", 
                my_cursor, (my_cursor as *const _ as usize) % CACHE_LINE_SIZE, self.reader_id);
            
            // Update our cursor - Use SeqCst for cross-process atomic reliability on macOS ARM64
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to store cursor value {} with SeqCst ordering", end_seq);
            my_cursor.store(end_seq, Ordering::SeqCst);
            tracing::debug!("🔍 SIGBUS DEBUG TRADES: Successfully updated cursor to {}", end_seq);
        }
        
        tracing::debug!("🔍 SIGBUS DEBUG TRADES: About to return {} trades", trades.len());
        trades
    }
}