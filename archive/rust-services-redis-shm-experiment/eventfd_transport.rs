// High-performance event-driven transport using eventfd
// This implements the architecture you specified - zero polling, lock-free, multi-consumer

use crate::{Result, AlphaPulseError};
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::mem::{size_of, align_of};
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::os::unix::io::{RawFd, AsRawFd};
use tracing::{info, debug, warn};

// Cross-platform event notification
#[cfg(target_os = "linux")]
use nix::sys::eventfd::{eventfd, EfdFlags};
use nix::unistd::{write, read};

const CACHE_LINE_SIZE: usize = 64;
const RING_SIZE: usize = 1_048_576; // 1M trades capacity
const MAX_CONSUMERS: usize = 8;

// Trade structure aligned to cache line
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct AlignedTrade {
    pub timestamp_ns: u64,
    pub symbol: [u8; 16],
    pub exchange: [u8; 16],
    pub price: f64,
    pub volume: f64,
    pub side: u8,  // 0=buy, 1=sell
    pub trade_id: [u8; 32],
    _padding: [u8; 7],  // Pad to 64 bytes
}

// Lock-free ring buffer with cache-line aligned atomics
#[repr(C)]
pub struct TradeRingBuffer {
    // Producer section (own cache line)
    write_head: CacheAligned<AtomicU64>,
    
    // Consumer section (separate cache lines)
    consumer_heads: [CacheAligned<AtomicU64>; MAX_CONSUMERS],
    
    // Metadata
    capacity: u64,
    mask: u64,  // For fast modulo
    
    // Trade data (bulk of memory)
    trades: [AlignedTrade; RING_SIZE],
}

// Helper to ensure cache line alignment
#[repr(C, align(64))]
struct CacheAligned<T> {
    value: T,
}

impl<T> CacheAligned<T> {
    fn new(value: T) -> Self {
        Self { value }
    }
}

// Event notification system - cross-platform
pub struct EventNotifier {
    #[cfg(target_os = "linux")]
    fd: RawFd,
    #[cfg(not(target_os = "linux"))]
    read_fd: RawFd,
    #[cfg(not(target_os = "linux"))]
    write_fd: RawFd,
}

impl EventNotifier {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            // Use eventfd on Linux
            let fd = eventfd(0, EfdFlags::EFD_CLOEXEC | EfdFlags::EFD_SEMAPHORE)
                .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to create eventfd: {}", e)))?;
            Ok(Self { fd })
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Use pipe on macOS/BSD
            let mut fds = [0i32; 2];
            let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
            if ret != 0 {
                return Err(AlphaPulseError::ConfigError(format!("Failed to create pipe: {}", std::io::Error::last_os_error())));
            }
            
            // Set non-blocking on write side to prevent blocking if pipe is full
            unsafe {
                let flags = libc::fcntl(fds[1], libc::F_GETFL);
                libc::fcntl(fds[1], libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
            
            Ok(Self {
                read_fd: fds[0],
                write_fd: fds[1],
            })
        }
    }
    
    /// Producer: Signal that N new items are available
    pub fn notify(&self, count: u64) -> Result<()> {
        let bytes = count.to_ne_bytes();
        
        #[cfg(target_os = "linux")]
        {
            write(self.fd, &bytes)
                .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to write to eventfd: {}", e)))?;
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            let ret = unsafe {
                libc::write(self.write_fd, bytes.as_ptr() as *const libc::c_void, 8)
            };
            if ret != 8 {
                return Err(AlphaPulseError::ConfigError(format!("Failed to write to pipe: {}", std::io::Error::last_os_error())));
            }
        }
        
        Ok(())
    }
    
    /// Consumer: Block until data is available, returns count
    pub fn wait(&self) -> Result<u64> {
        let mut buffer = [0u8; 8];
        
        #[cfg(target_os = "linux")]
        {
            read(self.fd, &mut buffer)
                .map_err(|e| AlphaPulseError::ConfigError(format!("Failed to read from eventfd: {}", e)))?;
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            let ret = unsafe {
                libc::read(self.read_fd, buffer.as_mut_ptr() as *mut libc::c_void, 8)
            };
            if ret != 8 {
                return Err(AlphaPulseError::ConfigError(format!("Failed to read from pipe: {}", std::io::Error::last_os_error())));
            }
        }
        
        Ok(u64::from_ne_bytes(buffer))
    }
    
    /// Get the raw file descriptor for epoll/select integration
    pub fn as_raw_fd(&self) -> RawFd {
        #[cfg(target_os = "linux")]
        { self.fd }
        
        #[cfg(not(target_os = "linux"))]
        { self.read_fd }
    }
}

impl Drop for EventNotifier {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            unsafe { libc::close(self.fd); }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            unsafe {
                libc::close(self.read_fd);
                libc::close(self.write_fd);
            }
        }
    }
}

// High-performance transport layer
pub struct AlphaPulseTransport {
    ring: *mut TradeRingBuffer,
    notifier: EventNotifier,
    mmap: MmapMut,
}

unsafe impl Send for AlphaPulseTransport {}
unsafe impl Sync for AlphaPulseTransport {}

impl AlphaPulseTransport {
    /// Create a new transport instance
    pub fn create(path: &str) -> Result<Self> {
        let total_size = size_of::<TradeRingBuffer>();
        info!("Creating AlphaPulse transport: {} MB", total_size / 1_048_576);
        
        // Create parent directory if needed
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Create memory-mapped file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        
        file.set_len(total_size as u64)?;
        
        let mut mmap = unsafe {
            MmapOptions::new()
                .len(total_size)
                .map_mut(&file)?
        };
        
        // Initialize ring buffer
        let ring_ptr = mmap.as_mut_ptr() as *mut TradeRingBuffer;
        unsafe {
            let ring = &mut *ring_ptr;
            
            // Initialize atomics
            ring.write_head = CacheAligned::new(AtomicU64::new(0));
            for i in 0..MAX_CONSUMERS {
                ring.consumer_heads[i] = CacheAligned::new(AtomicU64::new(0));
            }
            
            ring.capacity = RING_SIZE as u64;
            ring.mask = (RING_SIZE - 1) as u64;  // Works because RING_SIZE is power of 2
        }
        
        // Create event notifier
        let notifier = EventNotifier::new()?;
        
        info!("✅ AlphaPulse transport initialized with eventfd notification");
        
        Ok(Self {
            ring: ring_ptr,
            notifier,
            mmap,
        })
    }
    
    /// Producer: Write a batch of trades and notify consumers
    pub fn write_batch(&mut self, trades: &[AlignedTrade]) -> Result<usize> {
        unsafe {
            let ring = &mut *self.ring;
            let mut written = 0;
            
            for trade in trades {
                let write_pos = ring.write_head.value.load(Ordering::Relaxed);
                let next_write = write_pos.wrapping_add(1);
                
                // Check if ring is full (leave one slot empty to distinguish full/empty)
                let consumer_min = self.get_slowest_consumer();
                if next_write.wrapping_sub(consumer_min) >= ring.capacity {
                    warn!("Ring buffer full, dropping trades");
                    break;
                }
                
                // Write trade (zero-copy)
                let index = (write_pos & ring.mask) as usize;
                ring.trades[index] = *trade;
                
                // Publish write position (release ensures trade is visible)
                ring.write_head.value.store(next_write, Ordering::Release);
                written += 1;
            }
            
            // Notify consumers about new trades
            if written > 0 {
                self.notifier.notify(written as u64)?;
                debug!("Wrote {} trades and notified consumers", written);
            }
            
            Ok(written)
        }
    }
    
    /// Consumer: Read all available trades for a specific consumer
    pub fn read_trades(&mut self, consumer_id: usize) -> Result<Vec<AlignedTrade>> {
        if consumer_id >= MAX_CONSUMERS {
            return Err(AlphaPulseError::ConfigError(format!("Invalid consumer ID: {}", consumer_id)));
        }
        
        unsafe {
            let ring = &mut *self.ring;
            
            // Get current positions
            let mut read_pos = ring.consumer_heads[consumer_id].value.load(Ordering::Relaxed);
            let write_pos = ring.write_head.value.load(Ordering::Acquire);
            
            let mut trades = Vec::new();
            
            // Read all available trades
            while read_pos != write_pos {
                let index = (read_pos & ring.mask) as usize;
                trades.push(ring.trades[index]);
                read_pos = read_pos.wrapping_add(1);
            }
            
            // Update consumer position
            ring.consumer_heads[consumer_id].value.store(read_pos, Ordering::Release);
            
            Ok(trades)
        }
    }
    
    /// Wait for new trades (blocking)
    pub fn wait_for_trades(&self) -> Result<u64> {
        self.notifier.wait()
    }
    
    /// Get the file descriptor for async integration
    pub fn event_fd(&self) -> RawFd {
        self.notifier.as_raw_fd()
    }
    
    // Helper: Find the slowest consumer
    fn get_slowest_consumer(&self) -> u64 {
        unsafe {
            let ring = &*self.ring;
            let mut min_pos = u64::MAX;
            
            for i in 0..MAX_CONSUMERS {
                let pos = ring.consumer_heads[i].value.load(Ordering::Acquire);
                if pos < min_pos {
                    min_pos = pos;
                }
            }
            
            min_pos
        }
    }
}

// Consumer handle for reading trades
pub struct TransportConsumer {
    transport: std::sync::Arc<std::sync::Mutex<AlphaPulseTransport>>,
    consumer_id: usize,
}

impl TransportConsumer {
    pub fn new(transport: std::sync::Arc<std::sync::Mutex<AlphaPulseTransport>>, consumer_id: usize) -> Self {
        Self { transport, consumer_id }
    }
    
    /// Block until trades are available, then read them
    pub fn consume(&self) -> Result<Vec<AlignedTrade>> {
        // Wait for notification
        {
            let transport = self.transport.lock().unwrap();
            transport.wait_for_trades()?;
        }
        
        // Read available trades
        let mut transport = self.transport.lock().unwrap();
        transport.read_trades(self.consumer_id)
    }
    
    /// Try to read without blocking
    pub fn try_consume(&self) -> Result<Vec<AlignedTrade>> {
        let mut transport = self.transport.lock().unwrap();
        transport.read_trades(self.consumer_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_eventfd_notification() {
        let notifier = EventNotifier::new().unwrap();
        
        // Test in separate thread
        let notifier_clone = std::sync::Arc::new(notifier);
        let n1 = notifier_clone.clone();
        let n2 = notifier_clone.clone();
        
        let handle = thread::spawn(move || {
            let count = n1.wait().unwrap();
            assert_eq!(count, 42);
        });
        
        thread::sleep(Duration::from_millis(10));
        n2.notify(42).unwrap();
        
        handle.join().unwrap();
    }
    
    #[test]
    fn test_transport_basic() {
        let transport = AlphaPulseTransport::create("/tmp/test_transport").unwrap();
        assert!(transport.ring as usize != 0);
    }
}