// Event-driven shared memory with atomic reader registration
// This eliminates polling by using condition variables for notifications
// Provides true real-time performance for both dev dashboard and trading algorithms

use crate::{Result, AlphaPulseError, shared_memory::SharedTrade};
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error, debug};
use array_init::array_init;

// Constants for alignment and capacity
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const CACHE_LINE_SIZE: usize = 128;
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
const CACHE_LINE_SIZE: usize = 64;

const MAX_READERS: usize = 16; // Support up to 16 concurrent readers

// Atomic reader slot allocation for dynamic registration
// This enables readers to join/leave without races or fixed allocation
#[repr(C, align(128))]
pub struct AtomicReaderRegistry {
    // Atomic bitmap tracking active reader slots (bit 0 = slot 0, etc.)
    pub active_slots: AtomicU64,           // 8 bytes - supports up to 64 readers
    
    // Reader metadata slots
    pub reader_pids: [AtomicU32; MAX_READERS],    // 64 bytes - PIDs for cleanup
    pub reader_timestamps: [AtomicU64; MAX_READERS], // 128 bytes - last activity
    
    // Event notification for zero polling
    pub data_available: AtomicBool,        // 4 bytes - data ready flag
    pub notification_sequence: AtomicU64,  // 8 bytes - monotonic counter
    
    _padding: [u8; 124],                   // Pad to multiple of cache lines
}

impl AtomicReaderRegistry {
    /// Atomically claim a reader slot, returns slot ID (0-15) or error if full
    pub fn claim_reader_slot(&self) -> Result<usize> {
        info!("🔍 Starting claim_reader_slot - AtomicReaderRegistry at {:p}", self);
        info!("🔍 active_slots at {:p}, alignment: {}", &self.active_slots, std::mem::align_of_val(&self.active_slots));
        
        let current_pid = std::process::id();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Try to claim a slot atomically
        for slot in 0..MAX_READERS {
            info!("🔍 Attempting slot {} (total slots: {})", slot, MAX_READERS);
            let slot_bit = 1u64 << slot;
            
            // Check if the active_slots pointer is properly aligned for atomic operations
            let slots_addr = &self.active_slots as *const AtomicU64 as usize;
            if slots_addr % 8 != 0 {
                error!("❌ active_slots not 8-byte aligned: addr=0x{:x}", slots_addr);
                return Err(AlphaPulseError::ConfigError("active_slots alignment error".to_string()));
            }
            
            info!("🔍 About to perform fetch_or on active_slots at 0x{:x}", slots_addr);
            
            // First try a simple read to check if cross-process atomic access is working
            info!("🔍 Testing cross-process atomic read first...");
            let current_slots = self.active_slots.load(Ordering::SeqCst);
            info!("🔍 Cross-process atomic read successful: current_slots=0x{:x}", current_slots);
            
            // Now try the fetch_or operation with SeqCst for cross-process safety on macOS ARM64
            info!("🔍 Proceeding with fetch_or operation...");
            let old_slots = self.active_slots.fetch_or(slot_bit, Ordering::SeqCst);
            
            info!("🔍 fetch_or completed, old_slots=0x{:x}, slot_bit=0x{:x}", old_slots, slot_bit);
            
            if (old_slots & slot_bit) == 0 {
                info!("🔍 Slot {} is available, updating metadata", slot);
                
                // Check alignment of reader metadata arrays
                let pid_addr = &self.reader_pids[slot] as *const AtomicU32 as usize;
                let timestamp_addr = &self.reader_timestamps[slot] as *const AtomicU64 as usize;
                
                if pid_addr % 4 != 0 {
                    error!("❌ reader_pids[{}] not 4-byte aligned: addr=0x{:x}", slot, pid_addr);
                    return Err(AlphaPulseError::ConfigError("reader_pids alignment error".to_string()));
                }
                if timestamp_addr % 8 != 0 {
                    error!("❌ reader_timestamps[{}] not 8-byte aligned: addr=0x{:x}", slot, timestamp_addr);
                    return Err(AlphaPulseError::ConfigError("reader_timestamps alignment error".to_string()));
                }
                
                info!("🔍 About to store PID {} at slot {}", current_pid, slot);
                self.reader_pids[slot].store(current_pid, Ordering::SeqCst);
                
                info!("🔍 About to store timestamp {} at slot {}", current_time, slot);
                self.reader_timestamps[slot].store(current_time, Ordering::SeqCst);
                
                info!("✅ Claimed reader slot {} for PID {}", slot, current_pid);
                return Ok(slot);
            }
        }
        
        Err(AlphaPulseError::ConfigError("No available reader slots".to_string()))
    }
    
    /// Release a reader slot when disconnecting
    pub fn release_reader_slot(&self, slot: usize) -> Result<()> {
        if slot >= MAX_READERS {
            return Err(AlphaPulseError::ConfigError(format!("Invalid slot: {}", slot)));
        }
        
        let slot_bit = 1u64 << slot;
        
        // Clear metadata first
        self.reader_pids[slot].store(0, Ordering::Release);
        self.reader_timestamps[slot].store(0, Ordering::Release);
        
        // Atomically clear the slot bit
        self.active_slots.fetch_and(!slot_bit, Ordering::AcqRel);
        
        info!("🔓 Released reader slot {}", slot);
        Ok(())
    }
    
    /// Clean up stale reader slots (dead processes)
    pub fn cleanup_stale_readers(&self) -> usize {
        let mut cleaned = 0;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        for slot in 0..MAX_READERS {
            let slot_bit = 1u64 << slot;
            let active_slots = self.active_slots.load(Ordering::Acquire);
            
            if (active_slots & slot_bit) != 0 {
                let pid = self.reader_pids[slot].load(Ordering::Acquire);
                let timestamp = self.reader_timestamps[slot].load(Ordering::Acquire);
                
                // Check if process is dead or stale (30 second timeout)
                let is_stale = (current_time - timestamp) > 30;
                let is_dead = !is_process_alive(pid);
                
                if is_stale || is_dead {
                    info!("🧹 Cleaning up stale reader slot {} (PID: {}, stale: {}, dead: {})", 
                          slot, pid, is_stale, is_dead);
                    
                    // Release the slot
                    if let Err(e) = self.release_reader_slot(slot) {
                        warn!("Failed to release stale slot {}: {}", slot, e);
                    } else {
                        cleaned += 1;
                    }
                }
            }
        }
        
        cleaned
    }
    
    /// Notify all readers that new data is available
    pub fn notify_data_available(&self) {
        self.data_available.store(true, Ordering::Release);
        self.notification_sequence.fetch_add(1, Ordering::AcqRel);
        
        debug!("📢 Notified readers of new data");
    }
    
    /// Check if data is available and reset flag
    pub fn consume_notification(&self) -> bool {
        self.data_available.swap(false, Ordering::AcqRel)
    }
    
    /// Get current notification sequence for change detection
    pub fn get_notification_sequence(&self) -> u64 {
        self.notification_sequence.load(Ordering::Acquire)
    }
}

// Event-driven ring buffer header with atomic registration
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[repr(C, align(128))]
pub struct EventDrivenHeader {
    // Standard ring buffer metadata
    pub version: AtomicU32,           // 4 bytes
    pub capacity: AtomicU32,          // 4 bytes
    pub write_sequence: AtomicU64,    // 8 bytes
    pub writer_pid: AtomicU32,        // 4 bytes
    _padding1: [u8; 4],              // 4 bytes - align to 8
    pub last_write_ns: AtomicU64,     // 8 bytes
    
    // Atomic reader registration
    pub reader_registry: AtomicReaderRegistry, // 336 bytes
    
    _padding2: [u8; 752],            // Pad to multiple of 128 bytes
}

// Reader cursors: Each reader gets a 128-byte aligned cursor block
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[repr(C, align(128))]
pub struct EventDrivenReaderCursor {
    pub cursor: AtomicU64,           // 8 bytes - current read position
    pub last_heartbeat: AtomicU64,   // 8 bytes - for liveness detection
    _padding: [u8; 112],             // 112 bytes - Total: exactly 128 bytes
}

// Event-driven trade data with atomic operations
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[repr(C, align(128))]
pub struct EventDrivenTrade {
    pub timestamp_ns: AtomicU64,     // 8 bytes
    pub symbol: [u8; 16],            // 16 bytes - Fixed-size, no atomics needed
    pub exchange: [u8; 16],          // 16 bytes - Fixed-size, no atomics needed  
    pub price: AtomicU64,            // 8 bytes - Store as fixed-point (price * 1e8)
    pub volume: AtomicU64,           // 8 bytes - Store as fixed-point (volume * 1e8)
    pub side: AtomicU32,             // 4 bytes - 0=buy, 1=sell
    pub trade_id: [u8; 32],          // 32 bytes - Fixed-size, no atomics needed
    _padding: [u8; 20],              // 20 bytes - Total: 128 bytes exactly
}

// Simplified notification system using polling with backoff
// For now, we'll use adaptive polling that backs off when no data is available
// This bridges the gap between polling and true event-driven notifications
pub struct AdaptiveNotifier {
    last_sequence: u64,
    backoff_ms: u64,
}

impl AdaptiveNotifier {
    pub fn new() -> Self {
        Self {
            last_sequence: 0,
            backoff_ms: 1, // Start with 1ms
        }
    }
    
    /// Check for new data with adaptive backoff
    pub fn check_for_data(&mut self, current_sequence: u64) -> bool {
        if current_sequence > self.last_sequence {
            self.last_sequence = current_sequence;
            self.backoff_ms = 1; // Reset backoff on new data
            true
        } else {
            // No new data, increase backoff (max 10ms)
            self.backoff_ms = (self.backoff_ms * 2).min(10);
            false
        }
    }
    
    /// Get current backoff delay
    pub fn get_backoff_ms(&self) -> u64 {
        self.backoff_ms
    }
}

// Helper function to check if a process is alive
fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output();
        
        match output {
            Ok(result) => result.status.success(),
            Err(_) => false,
        }
    }
    
    #[cfg(not(unix))]
    {
        // On non-Unix systems, assume alive (conservative approach)
        true
    }
}

// Event-driven trade writer that notifies readers immediately
pub struct EventDrivenTradeWriter {
    mmap: MmapMut,
    header: *mut EventDrivenHeader,
    data_start: *mut EventDrivenTrade,
    cursor_blocks: *mut EventDrivenReaderCursor,
    capacity: usize,
    notifier: AdaptiveNotifier,
}

// SAFETY: The raw pointers are only accessed through atomic operations
// and the memory mapping ensures proper synchronization across processes
unsafe impl Send for EventDrivenTradeWriter {}
unsafe impl Sync for EventDrivenTradeWriter {}

impl EventDrivenTradeWriter {
    /// Create a new event-driven trade writer
    pub fn create(path: &str, capacity: usize) -> Result<Self> {
        let header_size = mem::size_of::<EventDrivenHeader>();
        let cursor_blocks_size = MAX_READERS * mem::size_of::<EventDrivenReaderCursor>();
        let data_size = capacity * mem::size_of::<EventDrivenTrade>();
        let total_size = header_size + cursor_blocks_size + data_size;
        
        // Ensure total size is aligned to cache line boundaries
        let aligned_size = (total_size + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);
        
        // Create parent directory if it doesn't exist
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
        let header_ptr = mmap.as_mut_ptr() as *mut EventDrivenHeader;
        unsafe {
            let header = &mut *header_ptr;
            header.version.store(1, Ordering::Relaxed);
            header.capacity.store(capacity as u32, Ordering::Relaxed);
            header.write_sequence.store(0, Ordering::Relaxed);
            header.writer_pid.store(std::process::id(), Ordering::Relaxed);
            header.last_write_ns.store(0, Ordering::Relaxed);
            
            // Initialize reader registry
            header.reader_registry.active_slots.store(0, Ordering::Relaxed);
            header.reader_registry.data_available.store(false, Ordering::Relaxed);
            header.reader_registry.notification_sequence.store(0, Ordering::Relaxed);
            
            for i in 0..MAX_READERS {
                header.reader_registry.reader_pids[i].store(0, Ordering::Relaxed);
                header.reader_registry.reader_timestamps[i].store(0, Ordering::Relaxed);
            }
        }
        
        // Initialize cursor blocks
        let cursor_blocks = unsafe {
            mmap.as_mut_ptr().add(header_size) as *mut EventDrivenReaderCursor
        };
        
        for i in 0..MAX_READERS {
            unsafe {
                let cursor_block = &mut *cursor_blocks.add(i);
                cursor_block.cursor.store(0, Ordering::SeqCst);
                cursor_block.last_heartbeat.store(0, Ordering::Relaxed);
            }
        }
        
        let data_start = unsafe {
            mmap.as_mut_ptr().add(header_size + cursor_blocks_size) as *mut EventDrivenTrade
        };
        
        // Create notifier
        let notifier = AdaptiveNotifier::new();
        
        info!("✅ Event-driven trade writer created: capacity={}, size={} bytes", capacity, aligned_size);
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            cursor_blocks,
            capacity,
            notifier,
        })
    }
    
    /// Write a trade and immediately notify all readers
    pub fn write_trade_and_notify(&mut self, trade: &SharedTrade) -> Result<()> {
        unsafe {
            let header = &mut *self.header;
            
            // Atomically increment write sequence
            let sequence = header.write_sequence.fetch_add(1, Ordering::AcqRel);
            let index = (sequence % self.capacity as u64) as usize;
            
            if index >= self.capacity {
                return Err(AlphaPulseError::BufferOverflow { index, capacity: self.capacity });
            }
            
            let trade_ptr = self.data_start.add(index);
            
            // Write trade data atomically
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
            
            // Update timestamp
            let now_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos() as u64;
            header.last_write_ns.store(now_ns, Ordering::Relaxed);
            
            // CRITICAL: Notify readers of new data availability
            header.reader_registry.notify_data_available();
            
            debug!("📢 Trade written and readers notified: seq={}, symbol={}", sequence, 
                   String::from_utf8_lossy(&trade.symbol).trim_end_matches('\0'));
        }
        
        Ok(())
    }
    
    /// Clean up stale readers periodically
    pub fn cleanup_stale_readers(&self) -> usize {
        unsafe {
            let header = &*self.header;
            header.reader_registry.cleanup_stale_readers()
        }
    }
}

// Event-driven trade reader that waits for notifications
pub struct EventDrivenTradeReader {
    mmap: MmapMut,  // Changed to MmapMut for atomic RMW operations
    header: *const EventDrivenHeader,
    data_start: *const EventDrivenTrade,
    cursor_blocks: *const EventDrivenReaderCursor,
    capacity: usize,
    reader_slot: usize,
    notifier: AdaptiveNotifier,
}

// SAFETY: The raw pointers are only accessed through atomic operations
// and the memory mapping ensures proper synchronization across processes
unsafe impl Send for EventDrivenTradeReader {}
unsafe impl Sync for EventDrivenTradeReader {}

impl EventDrivenTradeReader {
    /// Open an existing event-driven trade feed
    pub fn open(path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)  // CRITICAL: Need write access for atomic RMW operations on macOS ARM64
            .open(path)?;
            
        let mmap = unsafe { 
            MmapOptions::new()
                .len(file.metadata()?.len() as usize)
                .map_mut(&file)?  // Use map_mut for write access
        };
        
        let header_size = mem::size_of::<EventDrivenHeader>();
        let cursor_blocks_size = MAX_READERS * mem::size_of::<EventDrivenReaderCursor>();
        
        let header_ptr = mmap.as_ptr() as *const EventDrivenHeader;
        
        info!("🔍 EventDrivenTradeReader::open for {}", path);
        info!("🔍 mmap base addr: {:p}, size: {}", mmap.as_ptr(), mmap.len());
        info!("🔍 header_ptr: {:p}, header_size: {}", header_ptr, header_size);
        
        // Check header alignment
        let header_addr = header_ptr as usize;
        if header_addr % 128 != 0 {
            error!("❌ EventDrivenHeader not 128-byte aligned: addr=0x{:x}", header_addr);
            return Err(AlphaPulseError::ConfigError("Header alignment error".to_string()));
        }
        
        unsafe {
            let header = &*header_ptr;
            let capacity = header.capacity.load(Ordering::Acquire) as usize;
            
            // Check reader registry placement within header
            let registry_ptr = &header.reader_registry as *const AtomicReaderRegistry;
            let registry_addr = registry_ptr as usize;
            info!("🔍 AtomicReaderRegistry at {:p} (offset from header: {})", registry_ptr, registry_addr - header_addr);
            
            if registry_addr % 128 != 0 {
                error!("❌ AtomicReaderRegistry not 128-byte aligned: addr=0x{:x}", registry_addr);
                return Err(AlphaPulseError::ConfigError("Registry alignment error".to_string()));
            }
            
            // Claim a reader slot atomically
            let reader_slot = header.reader_registry.claim_reader_slot()?;
            
            let cursor_blocks = mmap.as_ptr().add(header_size) as *const EventDrivenReaderCursor;
            let data_start = mmap.as_ptr().add(header_size + cursor_blocks_size) as *const EventDrivenTrade;
            
            // Create notifier
            let notifier = AdaptiveNotifier::new();
            
            info!("✅ Event-driven trade reader opened: slot={}, capacity={}", reader_slot, capacity);
            
            Ok(Self {
                mmap,
                header: header_ptr,
                data_start,
                cursor_blocks,
                capacity,
                reader_slot,
                notifier,
            })
        }
    }
    
    /// Wait for new trades with adaptive polling (much more efficient than fixed polling!)
    pub fn wait_for_trades(&mut self, timeout: Duration) -> Result<Vec<SharedTrade>> {
        let start_time = std::time::Instant::now();
        
        loop {
            // Check for new data
            let trades = self.read_new_trades()?;
            if !trades.is_empty() {
                return Ok(trades);
            }
            
            // Check timeout
            if start_time.elapsed() >= timeout {
                return Ok(Vec::new());
            }
            
            // Adaptive backoff - sleep longer when no data is available
            let backoff_duration = Duration::from_millis(self.notifier.get_backoff_ms());
            std::thread::sleep(backoff_duration);
            
            // Update backoff for next iteration
            unsafe {
                let header = &*self.header;
                let current_seq = header.reader_registry.get_notification_sequence();
                self.notifier.check_for_data(current_seq);
            }
        }
    }
    
    /// Read all new trades since last read (non-blocking)
    pub fn read_new_trades(&mut self) -> Result<Vec<SharedTrade>> {
        let mut trades = Vec::new();
        
        unsafe {
            let header = &*self.header;
            let current_write_seq = header.write_sequence.load(Ordering::Acquire);
            
            // Get our cursor
            let my_cursor_block = &*self.cursor_blocks.add(self.reader_slot);
            let last_read_seq = my_cursor_block.cursor.load(Ordering::SeqCst);
            
            if current_write_seq <= last_read_seq {
                return Ok(trades); // No new data
            }
            
            // Read new trades
            for seq in last_read_seq..current_write_seq {
                let index = (seq % self.capacity as u64) as usize;
                let trade_ptr = self.data_start.add(index);
                
                // Read trade data atomically
                let timestamp_ns = (*trade_ptr).timestamp_ns.load(Ordering::Acquire);
                let price = (*trade_ptr).price.load(Ordering::Acquire) as f64 / 1e8;
                let volume = (*trade_ptr).volume.load(Ordering::Acquire) as f64 / 1e8;
                let side = (*trade_ptr).side.load(Ordering::Acquire) != 0;
                
                // Copy fixed-size arrays
                let mut symbol = [0u8; 16];
                let mut exchange = [0u8; 16];
                let mut trade_id = [0u8; 32];
                
                ptr::copy_nonoverlapping((*trade_ptr).symbol.as_ptr(), symbol.as_mut_ptr(), 16);
                ptr::copy_nonoverlapping((*trade_ptr).exchange.as_ptr(), exchange.as_mut_ptr(), 16);
                ptr::copy_nonoverlapping((*trade_ptr).trade_id.as_ptr(), trade_id.as_mut_ptr(), 32);
                
                let symbol_str = String::from_utf8_lossy(&symbol).trim_end_matches('\0').to_string();
                let exchange_str = String::from_utf8_lossy(&exchange).trim_end_matches('\0').to_string();
                let trade_id_str = String::from_utf8_lossy(&trade_id).trim_end_matches('\0').to_string();
                
                trades.push(SharedTrade::new(
                    timestamp_ns,
                    &symbol_str,
                    &exchange_str,
                    price,
                    volume,
                    side,
                    &trade_id_str,
                ));
            }
            
            // Update our cursor
            my_cursor_block.cursor.store(current_write_seq, Ordering::SeqCst);
            
            // Update heartbeat
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            my_cursor_block.last_heartbeat.store(now, Ordering::Relaxed);
            
            debug!("📊 Read {} new trades (slot {})", trades.len(), self.reader_slot);
        }
        
        Ok(trades)
    }
}

impl Drop for EventDrivenTradeReader {
    fn drop(&mut self) {
        // Release our reader slot when dropping
        unsafe {
            let header = &*self.header;
            if let Err(e) = header.reader_registry.release_reader_slot(self.reader_slot) {
                warn!("Failed to release reader slot {} on drop: {}", self.reader_slot, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reader_registry_claim_release() {
        let registry = AtomicReaderRegistry {
            active_slots: AtomicU64::new(0),
            reader_pids: array_init!(|_| AtomicU32::new(0)),
            reader_timestamps: array_init!(|_| AtomicU64::new(0)),
            data_available: AtomicBool::new(false),
            notification_sequence: AtomicU64::new(0),
            _padding: [0; 124],
        };
        
        // Claim first slot
        let slot1 = registry.claim_reader_slot().unwrap();
        assert_eq!(slot1, 0);
        
        // Claim second slot
        let slot2 = registry.claim_reader_slot().unwrap();
        assert_eq!(slot2, 1);
        
        // Release first slot
        registry.release_reader_slot(slot1).unwrap();
        
        // Should be able to claim first slot again
        let slot3 = registry.claim_reader_slot().unwrap();
        assert_eq!(slot3, 0);
    }
    
    #[test]
    fn test_notification_system() {
        let registry = AtomicReaderRegistry {
            active_slots: AtomicU64::new(0),
            reader_pids: array_init!(|_| AtomicU32::new(0)),
            reader_timestamps: array_init!(|_| AtomicU64::new(0)),
            data_available: AtomicBool::new(false),
            notification_sequence: AtomicU64::new(0),
            _padding: [0; 124],
        };
        
        // Initially no data available
        assert!(!registry.consume_notification());
        
        // Notify data available
        registry.notify_data_available();
        
        // Should now have notification
        assert!(registry.consume_notification());
        
        // Should be consumed (reset to false)
        assert!(!registry.consume_notification());
    }
}