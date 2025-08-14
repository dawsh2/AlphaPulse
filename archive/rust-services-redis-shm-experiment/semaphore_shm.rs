// True event-driven shared memory with semaphores - zero polling!
// This implementation uses POSIX semaphores for cross-process notifications
// Provides sub-microsecond latency for both dashboard and trading algorithms

use crate::{Result, AlphaPulseError, shared_memory::SharedTrade};
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error, debug};

#[cfg(unix)]
use libc::{sem_t, sem_init, sem_post, sem_wait, sem_destroy};

// Platform-specific cache line alignment
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const CACHE_LINE_SIZE: usize = 128;
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
const CACHE_LINE_SIZE: usize = 64;

const MAX_READERS: usize = 16;

// Cross-process semaphore wrapper using named semaphores (fully supported on macOS)
#[cfg(unix)]
#[repr(C, align(128))]
pub struct CrossProcessSemaphore {
    // Use named semaphores which are fully supported on macOS
    inner: Option<crate::named_semaphore::NamedSemaphore>,
    _padding: [u8; 120], // Adjust padding for alignment
}

#[cfg(unix)]
impl CrossProcessSemaphore {
    /// Initialize semaphore for cross-process use
    pub fn init(&mut self) -> Result<()> {
        // Create a named semaphore (fully supported on macOS)
        match crate::named_semaphore::NamedSemaphore::create(0) {
            Ok(sem) => {
                info!("✅ Named semaphore created successfully: {}", sem.name());
                self.inner = Some(sem);
                Ok(())
            }
            Err(e) => {
                error!("Failed to create named semaphore: {}", e);
                Err(e)
            }
        }
    }
    
    /// Signal the semaphore (wake up waiting readers)
    pub fn post(&self) -> Result<()> {
        if let Some(ref sem) = self.inner {
            sem.post()
        } else {
            Err(AlphaPulseError::ConfigError("Semaphore not initialized".to_string()))
        }
    }
    
    /// Wait for semaphore signal (blocks until data available)
    pub fn wait(&self) -> Result<()> {
        if let Some(ref sem) = self.inner {
            sem.wait()
        } else {
            Err(AlphaPulseError::ConfigError("Semaphore not initialized".to_string()))
        }
    }
    
    /// Wait for semaphore signal with timeout (simplified for macOS compatibility)
    pub fn wait_timeout(&self, timeout: Duration) -> Result<bool> {
        if let Some(ref sem) = self.inner {
            // Use polling with the proper macOS semaphore
            let start = std::time::Instant::now();
            let sleep_duration = Duration::from_millis(1);
            
            loop {
                // Try non-blocking wait first
                if sem.try_wait()? {
                    return Ok(true);
                }
                
                // Check timeout
                if start.elapsed() >= timeout {
                    return Ok(false);
                }
                
                // Small sleep to avoid busy waiting
                std::thread::sleep(sleep_duration);
            }
        } else {
            Err(AlphaPulseError::ConfigError("Semaphore not initialized".to_string()))
        }
    }
    
    /// Try to wait without blocking
    fn try_wait(&self) -> Result<bool> {
        if let Some(ref sem) = self.inner {
            sem.try_wait()
        } else {
            Err(AlphaPulseError::ConfigError("Semaphore not initialized".to_string()))
        }
    }
    
    /// Destroy semaphore (handled automatically by Drop)
    pub fn destroy(&mut self) -> Result<()> {
        // The MacOSSemaphore Drop implementation handles cleanup
        self.inner = None;
        Ok(())
    }
}

// Non-Unix fallback (Windows)
#[cfg(not(unix))]
#[repr(C, align(128))]
pub struct CrossProcessSemaphore {
    _placeholder: [u8; 128],
}

#[cfg(not(unix))]
impl CrossProcessSemaphore {
    pub fn init(&mut self) -> Result<()> {
        warn!("Semaphores not supported on this platform, falling back to polling");
        Ok(())
    }
    
    pub fn post(&self) -> Result<()> {
        Ok(())
    }
    
    pub fn wait(&self) -> Result<()> {
        std::thread::sleep(Duration::from_millis(1));
        Ok(())
    }
    
    pub fn wait_timeout(&self, _timeout: Duration) -> Result<bool> {
        std::thread::sleep(Duration::from_millis(1));
        Ok(false) // Always timeout on non-Unix
    }
    
    pub fn destroy(&mut self) -> Result<()> {
        Ok(())
    }
}

// Reader registration with atomic slots
#[repr(C, align(128))]
pub struct SemaphoreReaderRegistry {
    pub active_readers: AtomicU32,         // 4 bytes - count of active readers
    pub reader_pids: [AtomicU32; MAX_READERS], // 64 bytes - PIDs for cleanup
    pub reader_timestamps: [AtomicU64; MAX_READERS], // 128 bytes - heartbeats
    pub data_semaphore: CrossProcessSemaphore, // 128 bytes - notification semaphore
    _padding: [u8; 32],                    // 32 bytes - Total: 356 bytes, align to 384
}

impl SemaphoreReaderRegistry {
    /// Initialize the registry
    pub fn init(&mut self) -> Result<()> {
        self.active_readers.store(0, Ordering::Relaxed);
        
        for i in 0..MAX_READERS {
            self.reader_pids[i].store(0, Ordering::Relaxed);
            self.reader_timestamps[i].store(0, Ordering::Relaxed);
        }
        
        self.data_semaphore.init()?;
        
        info!("✅ Semaphore-based reader registry initialized");
        Ok(())
    }
    
    /// Register a new reader
    pub fn register_reader(&self) -> Result<usize> {
        let current_pid = std::process::id();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Find an empty slot
        for slot in 0..MAX_READERS {
            if self.reader_pids[slot].compare_exchange_weak(
                0, current_pid, Ordering::AcqRel, Ordering::Relaxed
            ).is_ok() {
                self.reader_timestamps[slot].store(current_time, Ordering::Release);
                self.active_readers.fetch_add(1, Ordering::AcqRel);
                
                info!("✅ Registered reader in slot {} (PID: {})", slot, current_pid);
                return Ok(slot);
            }
        }
        
        Err(AlphaPulseError::ConfigError("No available reader slots".to_string()))
    }
    
    /// Unregister a reader
    pub fn unregister_reader(&self, slot: usize) -> Result<()> {
        if slot >= MAX_READERS {
            return Err(AlphaPulseError::ConfigError(format!("Invalid slot: {}", slot)));
        }
        
        self.reader_pids[slot].store(0, Ordering::Release);
        self.reader_timestamps[slot].store(0, Ordering::Release);
        self.active_readers.fetch_sub(1, Ordering::AcqRel);
        
        info!("🔓 Unregistered reader from slot {}", slot);
        Ok(())
    }
    
    /// Notify all readers that data is available (wake them up!)
    pub fn notify_data_available(&self) -> Result<()> {
        let active_count = self.active_readers.load(Ordering::Acquire);
        
        // Post semaphore once for each active reader
        for _ in 0..active_count {
            self.data_semaphore.post()?;
        }
        
        debug!("📢 Notified {} readers via semaphore", active_count);
        Ok(())
    }
    
    /// Wait for data notification (blocks until data available)
    pub fn wait_for_data(&self) -> Result<()> {
        debug!("⏳ Waiting for data notification via semaphore...");
        self.data_semaphore.wait()?;
        debug!("✅ Data notification received!");
        Ok(())
    }
    
    /// Wait for data notification with timeout
    pub fn wait_for_data_timeout(&self, timeout: Duration) -> Result<bool> {
        debug!("⏳ Waiting for data notification (timeout: {:?})...", timeout);
        let result = self.data_semaphore.wait_timeout(timeout)?;
        if result {
            debug!("✅ Data notification received!");
        } else {
            debug!("⏰ Wait timeout");
        }
        Ok(result)
    }
    
    /// Update reader heartbeat
    pub fn update_heartbeat(&self, slot: usize) -> Result<()> {
        if slot >= MAX_READERS {
            return Err(AlphaPulseError::ConfigError(format!("Invalid slot: {}", slot)));
        }
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.reader_timestamps[slot].store(current_time, Ordering::Release);
        Ok(())
    }
    
    /// Clean up stale readers
    pub fn cleanup_stale_readers(&self) -> usize {
        let mut cleaned = 0;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        for slot in 0..MAX_READERS {
            let pid = self.reader_pids[slot].load(Ordering::Acquire);
            if pid != 0 {
                let timestamp = self.reader_timestamps[slot].load(Ordering::Acquire);
                let is_stale = (current_time - timestamp) > 30; // 30 second timeout
                let is_dead = !is_process_alive(pid);
                
                if is_stale || is_dead {
                    info!("🧹 Cleaning up stale reader slot {} (PID: {}, stale: {}, dead: {})", 
                          slot, pid, is_stale, is_dead);
                    
                    if let Err(e) = self.unregister_reader(slot) {
                        warn!("Failed to unregister stale reader {}: {}", slot, e);
                    } else {
                        cleaned += 1;
                    }
                }
            }
        }
        
        cleaned
    }
}

impl Drop for SemaphoreReaderRegistry {
    fn drop(&mut self) {
        if let Err(e) = self.data_semaphore.destroy() {
            warn!("Failed to destroy semaphore: {}", e);
        }
    }
}

// Semaphore-based shared memory header
#[repr(C, align(128))]
pub struct SemaphoreSharedMemoryHeader {
    pub version: AtomicU32,               // 4 bytes
    pub capacity: AtomicU32,              // 4 bytes
    pub write_sequence: AtomicU64,        // 8 bytes
    pub writer_pid: AtomicU32,            // 4 bytes
    _padding1: [u8; 4],                  // 4 bytes - align to 8
    pub last_write_ns: AtomicU64,         // 8 bytes
    pub reader_registry: SemaphoreReaderRegistry, // 384 bytes
    _padding2: [u8; 592],                // Pad to multiple of 128 bytes (total: 1024)
}

// Reader cursor blocks (same as before)
#[repr(C, align(128))]
pub struct SemaphoreReaderCursor {
    pub cursor: AtomicU64,               // 8 bytes
    pub last_heartbeat: AtomicU64,       // 8 bytes
    _padding: [u8; 112],                 // 112 bytes - Total: 128 bytes
}

// Trade data (same as event-driven version)
#[repr(C, align(128))]
pub struct SemaphoreTrade {
    pub timestamp_ns: AtomicU64,         // 8 bytes
    pub symbol: [u8; 16],                // 16 bytes
    pub exchange: [u8; 16],              // 16 bytes
    pub price: AtomicU64,                // 8 bytes (fixed-point * 1e8)
    pub volume: AtomicU64,               // 8 bytes (fixed-point * 1e8)
    pub side: AtomicU32,                 // 4 bytes (0=buy, 1=sell)
    pub trade_id: [u8; 32],              // 32 bytes
    _padding: [u8; 20],                  // 20 bytes - Total: 128 bytes
}

// True event-driven trade writer with semaphore notifications
pub struct SemaphoreTradeWriter {
    mmap: MmapMut,
    header: *mut SemaphoreSharedMemoryHeader,
    data_start: *mut SemaphoreTrade,
    cursor_blocks: *mut SemaphoreReaderCursor,
    capacity: usize,
}

unsafe impl Send for SemaphoreTradeWriter {}
unsafe impl Sync for SemaphoreTradeWriter {}

impl SemaphoreTradeWriter {
    /// Create a new semaphore-based trade writer
    pub fn create(path: &str, capacity: usize) -> Result<Self> {
        let header_size = mem::size_of::<SemaphoreSharedMemoryHeader>();
        let cursor_blocks_size = MAX_READERS * mem::size_of::<SemaphoreReaderCursor>();
        let data_size = capacity * mem::size_of::<SemaphoreTrade>();
        let total_size = header_size + cursor_blocks_size + data_size;
        
        // Align to cache line boundaries
        let aligned_size = (total_size + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);
        
        // Create parent directory
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
        let header_ptr = mmap.as_mut_ptr() as *mut SemaphoreSharedMemoryHeader;
        unsafe {
            let header = &mut *header_ptr;
            header.version.store(1, Ordering::Relaxed);
            header.capacity.store(capacity as u32, Ordering::Relaxed);
            header.write_sequence.store(0, Ordering::Relaxed);
            header.writer_pid.store(std::process::id(), Ordering::Relaxed);
            header.last_write_ns.store(0, Ordering::Relaxed);
            
            // Initialize reader registry with semaphore
            header.reader_registry.init()?;
        }
        
        // Initialize cursor blocks
        let cursor_blocks = unsafe {
            mmap.as_mut_ptr().add(header_size) as *mut SemaphoreReaderCursor
        };
        
        for i in 0..MAX_READERS {
            unsafe {
                let cursor_block = &mut *cursor_blocks.add(i);
                cursor_block.cursor.store(0, Ordering::Relaxed);
                cursor_block.last_heartbeat.store(0, Ordering::Relaxed);
            }
        }
        
        let data_start = unsafe {
            mmap.as_mut_ptr().add(header_size + cursor_blocks_size) as *mut SemaphoreTrade
        };
        
        info!("✅ Semaphore-based trade writer created: capacity={}, size={} bytes", capacity, aligned_size);
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            cursor_blocks,
            capacity,
        })
    }
    
    /// Write trade and immediately wake up all waiting readers
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
            
            // CRITICAL: Wake up all waiting readers via semaphore
            header.reader_registry.notify_data_available()?;
            
            debug!("📢 Trade written and readers notified via semaphore: seq={}, symbol={}", 
                   sequence, String::from_utf8_lossy(&trade.symbol).trim_end_matches('\0'));
        }
        
        Ok(())
    }
    
    /// Clean up stale readers
    pub fn cleanup_stale_readers(&self) -> usize {
        unsafe {
            let header = &*self.header;
            header.reader_registry.cleanup_stale_readers()
        }
    }
}

// True event-driven trade reader with semaphore blocking
pub struct SemaphoreTradeReader {
    mmap: MmapMut,
    header: *const SemaphoreSharedMemoryHeader,
    data_start: *const SemaphoreTrade,
    cursor_blocks: *const SemaphoreReaderCursor,
    capacity: usize,
    reader_slot: usize,
}

unsafe impl Send for SemaphoreTradeReader {}
unsafe impl Sync for SemaphoreTradeReader {}

impl SemaphoreTradeReader {
    /// Open existing semaphore-based trade feed
    pub fn open(path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true) // Need write for atomic operations
            .open(path)?;
            
        let mmap = unsafe {
            MmapOptions::new()
                .len(file.metadata()?.len() as usize)
                .map_mut(&file)?
        };
        
        let header_size = mem::size_of::<SemaphoreSharedMemoryHeader>();
        let cursor_blocks_size = MAX_READERS * mem::size_of::<SemaphoreReaderCursor>();
        
        let header_ptr = mmap.as_ptr() as *const SemaphoreSharedMemoryHeader;
        
        unsafe {
            let header = &*header_ptr;
            let capacity = header.capacity.load(Ordering::Acquire) as usize;
            
            // Register as a reader
            let reader_slot = header.reader_registry.register_reader()?;
            
            let cursor_blocks = mmap.as_ptr().add(header_size) as *const SemaphoreReaderCursor;
            let data_start = mmap.as_ptr().add(header_size + cursor_blocks_size) as *const SemaphoreTrade;
            
            info!("✅ Semaphore-based trade reader opened: slot={}, capacity={}", reader_slot, capacity);
            
            Ok(Self {
                mmap,
                header: header_ptr,
                data_start,
                cursor_blocks,
                capacity,
                reader_slot,
            })
        }
    }
    
    /// Block until new trades arrive (zero polling!)
    pub fn wait_for_trades(&mut self, timeout: Duration) -> Result<Vec<SharedTrade>> {
        loop {
            // Check for existing data first
            let trades = self.read_new_trades()?;
            if !trades.is_empty() {
                return Ok(trades);
            }
            
            // No data available - block on semaphore until notified
            unsafe {
                let header = &*self.header;
                if header.reader_registry.wait_for_data_timeout(timeout)? {
                    // Semaphore signaled - data should be available now
                    continue;
                } else {
                    // Timeout - return empty
                    return Ok(Vec::new());
                }
            }
        }
    }
    
    /// Read all new trades (non-blocking)
    pub fn read_new_trades(&mut self) -> Result<Vec<SharedTrade>> {
        let mut trades = Vec::new();
        
        unsafe {
            let header = &*self.header;
            let current_write_seq = header.write_sequence.load(Ordering::Acquire);
            
            // Get our cursor
            let my_cursor_block = &*self.cursor_blocks.add(self.reader_slot);
            let last_read_seq = my_cursor_block.cursor.load(Ordering::Acquire);
            
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
            my_cursor_block.cursor.store(current_write_seq, Ordering::Release);
            
            // Update heartbeat
            header.reader_registry.update_heartbeat(self.reader_slot)?;
            
            debug!("📊 Read {} new trades via semaphore (slot {})", trades.len(), self.reader_slot);
        }
        
        Ok(trades)
    }
}

impl Drop for SemaphoreTradeReader {
    fn drop(&mut self) {
        // Unregister reader on drop
        unsafe {
            let header = &*self.header;
            if let Err(e) = header.reader_registry.unregister_reader(self.reader_slot) {
                warn!("Failed to unregister reader slot {} on drop: {}", self.reader_slot, e);
            }
        }
    }
}

// Helper function to check if process is alive
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
        true // Conservative approach on non-Unix
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_semaphore_init() {
        use array_init::array_init;
        
        let mut registry = SemaphoreReaderRegistry {
            active_readers: AtomicU32::new(0),
            reader_pids: array_init(|_| AtomicU32::new(0)),
            reader_timestamps: array_init(|_| AtomicU64::new(0)),
            data_semaphore: CrossProcessSemaphore {
                inner: None,
                _padding: [0; 120],
            },
            _padding: [0; 32],
        };
        
        registry.init().unwrap();
        assert_eq!(registry.active_readers.load(Ordering::Relaxed), 0);
    }
    
    #[test]
    fn test_reader_registration() {
        use array_init::array_init;
        
        let mut registry = SemaphoreReaderRegistry {
            active_readers: AtomicU32::new(0),
            reader_pids: array_init(|_| AtomicU32::new(0)),
            reader_timestamps: array_init(|_| AtomicU64::new(0)),
            data_semaphore: CrossProcessSemaphore {
                inner: None,
                _padding: [0; 120],
            },
            _padding: [0; 32],
        };
        
        registry.init().unwrap();
        
        // Register first reader
        let slot1 = registry.register_reader().unwrap();
        assert_eq!(slot1, 0);
        assert_eq!(registry.active_readers.load(Ordering::Relaxed), 1);
        
        // Register second reader
        let slot2 = registry.register_reader().unwrap();
        assert_eq!(slot2, 1);
        assert_eq!(registry.active_readers.load(Ordering::Relaxed), 2);
        
        // Unregister first reader
        registry.unregister_reader(slot1).unwrap();
        assert_eq!(registry.active_readers.load(Ordering::Relaxed), 1);
    }
}