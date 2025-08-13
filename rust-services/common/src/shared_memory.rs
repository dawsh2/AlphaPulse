// Shared memory implementation for ultra-low latency IPC
// Uses lock-free ring buffers in /tmp for < 10μs latency

use std::sync::atomic::{AtomicU64, Ordering};
use std::ptr;
use std::mem;
use std::fs::{OpenOptions, File};
use std::path::Path;
use memmap2::{MmapMut, MmapOptions};
use serde::{Serialize, Deserialize};
use crate::{Result, AlphaPulseError};

// Fixed-size trade struct for zero-copy operations
// Aligned to 128 bytes for cache line efficiency
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SharedTrade {
    pub timestamp_ns: u64,           // 8 bytes
    pub symbol: [u8; 16],            // 16 bytes  
    pub exchange: [u8; 16],          // 16 bytes
    pub price: f64,                  // 8 bytes
    pub volume: f64,                 // 8 bytes
    pub side: u8,                    // 1 byte (0=buy, 1=sell)
    pub trade_id: [u8; 32],          // 32 bytes
    _padding: [u8; 39],              // 39 bytes padding to reach 128 bytes
}

impl SharedTrade {
    pub const SIZE: usize = 128;
    
    pub fn new(
        timestamp_ns: u64,
        symbol: &str,
        exchange: &str,
        price: f64,
        volume: f64,
        side: bool,  // true = buy, false = sell
        trade_id: &str,
    ) -> Self {
        let mut trade = Self {
            timestamp_ns,
            symbol: [0; 16],
            exchange: [0; 16],
            price,
            volume,
            side: if side { 0 } else { 1 },
            trade_id: [0; 32],
            _padding: [0; 39],
        };
        
        // Copy strings into fixed-size arrays
        let symbol_bytes = symbol.as_bytes();
        let exchange_bytes = exchange.as_bytes();
        let trade_id_bytes = trade_id.as_bytes();
        
        trade.symbol[..symbol_bytes.len().min(16)].copy_from_slice(
            &symbol_bytes[..symbol_bytes.len().min(16)]
        );
        trade.exchange[..exchange_bytes.len().min(16)].copy_from_slice(
            &exchange_bytes[..exchange_bytes.len().min(16)]
        );
        trade.trade_id[..trade_id_bytes.len().min(32)].copy_from_slice(
            &trade_id_bytes[..trade_id_bytes.len().min(32)]
        );
        
        trade
    }
    
    pub fn symbol_str(&self) -> String {
        String::from_utf8_lossy(&self.symbol)
            .trim_end_matches('\0')
            .to_string()
    }
    
    pub fn exchange_str(&self) -> String {
        String::from_utf8_lossy(&self.exchange)
            .trim_end_matches('\0')
            .to_string()
    }
}

// Fixed-size orderbook delta struct for zero-copy operations
// Aligned to 256 bytes for cache line efficiency
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SharedOrderBookDelta {
    pub timestamp_ns: u64,           // 8 bytes
    pub symbol: [u8; 16],            // 16 bytes
    pub exchange: [u8; 16],          // 16 bytes
    pub version: u64,                // 8 bytes
    pub prev_version: u64,           // 8 bytes
    pub change_count: u16,           // 2 bytes - number of price level changes
    _padding_after_count: [u8; 2],   // 2 bytes padding for alignment
    pub changes: [PriceLevelChange; 16], // 16 changes * 12 bytes = 192 bytes
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PriceLevelChange {
    pub price: f32,                  // 4 bytes (sufficient precision for prices)
    pub volume: f32,                 // 4 bytes (sufficient for volume)
    pub side_and_action: u8,         // 1 byte: high bit = side (0=bid,1=ask), low 7 bits = action
    _padding: [u8; 3],               // 3 bytes padding to reach 12 bytes
}

impl SharedOrderBookDelta {
    pub const SIZE: usize = 256;
    pub const MAX_CHANGES: usize = 16;
    
    pub fn new(
        timestamp_ns: u64,
        symbol: &str,
        exchange: &str,
        version: u64,
        prev_version: u64,
    ) -> Self {
        let mut delta = Self {
            timestamp_ns,
            symbol: [0; 16],
            exchange: [0; 16],
            version,
            prev_version,
            change_count: 0,
            _padding_after_count: [0; 2],
            changes: [PriceLevelChange::default(); 16],
        };
        
        // Copy strings into fixed-size arrays
        let symbol_bytes = symbol.as_bytes();
        let exchange_bytes = exchange.as_bytes();
        
        delta.symbol[..symbol_bytes.len().min(16)].copy_from_slice(
            &symbol_bytes[..symbol_bytes.len().min(16)]
        );
        delta.exchange[..exchange_bytes.len().min(16)].copy_from_slice(
            &exchange_bytes[..exchange_bytes.len().min(16)]
        );
        
        delta
    }
    
    pub fn add_change(&mut self, price: f64, volume: f64, is_ask: bool, action: u8) -> bool {
        if self.change_count as usize >= Self::MAX_CHANGES {
            return false; // Buffer full
        }
        
        let side_and_action = if is_ask { 0x80 | action } else { action };
        
        self.changes[self.change_count as usize] = PriceLevelChange {
            price: price as f32,
            volume: volume as f32,
            side_and_action,
            _padding: [0; 3],
        };
        self.change_count += 1;
        true
    }
    
    pub fn symbol_str(&self) -> String {
        String::from_utf8_lossy(&self.symbol)
            .trim_end_matches('\0')
            .to_string()
    }
    
    pub fn exchange_str(&self) -> String {
        String::from_utf8_lossy(&self.exchange)
            .trim_end_matches('\0')
            .to_string()
    }
}

impl Default for PriceLevelChange {
    fn default() -> Self {
        Self {
            price: 0.0f32,
            volume: 0.0f32,
            side_and_action: 0,
            _padding: [0; 3],
        }
    }
}

// Ring buffer header for coordination between writers and readers
#[repr(C)]
pub struct RingBufferHeader {
    pub version: u32,
    pub capacity: u32,
    pub write_sequence: AtomicU64,
    pub cached_write_sequence: u64,  // Non-atomic cached value for readers
    pub writer_pid: u32,
    pub last_write_ns: AtomicU64,
    pub reader_cursors: [AtomicU64; 16],  // Support up to 16 readers
    _padding: [u8; 64],  // Pad to cache line
}

pub struct SharedMemoryWriter {
    mmap: MmapMut,
    header: *mut RingBufferHeader,
    data_start: *mut u8,
    capacity: usize,
}

unsafe impl Send for SharedMemoryWriter {}
unsafe impl Sync for SharedMemoryWriter {}

impl SharedMemoryWriter {
    pub fn create(path: &str, capacity: usize) -> Result<Self> {
        // Validate memory layout at compile time
        Self::validate_memory_layout()?;
        
        // Validate capacity bounds
        if capacity == 0 {
            return Err(AlphaPulseError::ConfigError("Capacity must be greater than 0".to_string()));
        }
        if capacity > 1_000_000 {
            return Err(AlphaPulseError::ConfigError("Capacity too large (max 1M)".to_string()));
        }
        
        // Calculate total size: header + (capacity * trade_size)
        let header_size = mem::size_of::<RingBufferHeader>();
        let total_size = header_size + (capacity * SharedTrade::SIZE);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Create or open the shared memory file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        
        // Set file size
        file.set_len(total_size as u64)?;
        
        // Memory map the file
        let mut mmap = unsafe {
            MmapOptions::new()
                .len(total_size)
                .map_mut(&file)?
        };
        
        // Initialize header
        let header_ptr = mmap.as_mut_ptr() as *mut RingBufferHeader;
        unsafe {
            let header = &mut *header_ptr;
            header.version = 1;
            header.capacity = capacity as u32;
            header.write_sequence.store(0, Ordering::Relaxed);
            header.cached_write_sequence = 0;
            header.writer_pid = std::process::id();
            header.last_write_ns.store(0, Ordering::Relaxed);
            
            // Initialize reader cursors
            for cursor in &header.reader_cursors {
                cursor.store(0, Ordering::Relaxed);
            }
        }
        
        // Calculate data start position
        let data_start = unsafe {
            mmap.as_mut_ptr().add(header_size)
        };
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            capacity,
        })
    }
    
    pub fn write_trade(&mut self, trade: &SharedTrade) -> Result<()> {
        unsafe {
            // Validate header pointer before dereferencing
            if self.header.is_null() {
                return Err(AlphaPulseError::MemoryCorruption);
            }
            
            let header = &mut *self.header;
            
            // Get current write position
            let sequence = header.write_sequence.fetch_add(1, Ordering::AcqRel);
            let index = (sequence % self.capacity as u64) as usize;
            
            // Bounds checking - critical for memory safety
            if index >= self.capacity {
                return Err(AlphaPulseError::BufferOverflow { 
                    index, 
                    capacity: self.capacity 
                });
            }
            
            // Calculate trade position in buffer with bounds validation
            let offset = index * SharedTrade::SIZE;
            if self.data_start.is_null() {
                return Err(AlphaPulseError::MemoryCorruption);
            }
            
            let trade_ptr = self.data_start.add(offset) as *mut SharedTrade;
            
            // Validate pointer alignment
            if (trade_ptr as usize) % mem::align_of::<SharedTrade>() != 0 {
                return Err(AlphaPulseError::MemoryCorruption);
            }
            
            // Write trade with volatile semantics to prevent reordering
            ptr::write_volatile(trade_ptr, *trade);
            
            // Memory fence to ensure write is visible
            std::sync::atomic::fence(Ordering::Release);
            
            // Update timestamp with proper error handling
            let now_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(AlphaPulseError::SystemTimeError)?
                .as_nanos() as u64;
            header.last_write_ns.store(now_ns, Ordering::Relaxed);
            
            // Update cached sequence for readers
            header.cached_write_sequence = sequence + 1;
        }
        
        Ok(())
    }
    
    pub fn get_stats(&self) -> (u64, u64) {
        unsafe {
            let header = &*self.header;
            let writes = header.write_sequence.load(Ordering::Relaxed);
            let last_write = header.last_write_ns.load(Ordering::Relaxed);
            (writes, last_write)
        }
    }
    
    fn validate_memory_layout() -> Result<()> {
        // Verify struct sizes at compile time
        let trade_size = mem::size_of::<SharedTrade>();
        let delta_size = mem::size_of::<SharedOrderBookDelta>();
        let header_size = mem::size_of::<RingBufferHeader>();
        
        if trade_size != SharedTrade::SIZE {
            return Err(AlphaPulseError::InvalidMemoryLayout {
                expected: SharedTrade::SIZE,
                actual: trade_size,
            });
        }
        
        if delta_size != SharedOrderBookDelta::SIZE {
            return Err(AlphaPulseError::InvalidMemoryLayout {
                expected: SharedOrderBookDelta::SIZE,
                actual: delta_size,
            });
        }
        
        // Verify alignment requirements
        if header_size % mem::align_of::<RingBufferHeader>() != 0 {
            return Err(AlphaPulseError::MemoryMappingError(
                "Header alignment mismatch".to_string()
            ));
        }
        
        Ok(())
    }
}

// Specialized writer for orderbook deltas
pub struct OrderBookDeltaWriter {
    mmap: MmapMut,
    header: *mut RingBufferHeader,
    data_start: *mut u8,
    capacity: usize,
}

unsafe impl Send for OrderBookDeltaWriter {}
unsafe impl Sync for OrderBookDeltaWriter {}

impl OrderBookDeltaWriter {
    pub fn create(path: &str, capacity: usize) -> Result<Self> {
        // Calculate total size: header + (capacity * delta_size)
        let header_size = mem::size_of::<RingBufferHeader>();
        let total_size = header_size + (capacity * SharedOrderBookDelta::SIZE);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Create or open the shared memory file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        
        // Set file size
        file.set_len(total_size as u64)?;
        
        // Memory map the file
        let mut mmap = unsafe {
            MmapOptions::new()
                .len(total_size)
                .map_mut(&file)?
        };
        
        // Initialize header
        let header_ptr = mmap.as_mut_ptr() as *mut RingBufferHeader;
        unsafe {
            let header = &mut *header_ptr;
            header.version = 1;
            header.capacity = capacity as u32;
            header.write_sequence.store(0, Ordering::Relaxed);
            header.cached_write_sequence = 0;
            header.writer_pid = std::process::id();
            header.last_write_ns.store(0, Ordering::Relaxed);
            
            // Initialize reader cursors
            for cursor in &header.reader_cursors {
                cursor.store(0, Ordering::Relaxed);
            }
        }
        
        // Calculate data start position
        let data_start = unsafe {
            mmap.as_mut_ptr().add(header_size)
        };
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            capacity,
        })
    }
    
    pub fn write_delta(&mut self, delta: &SharedOrderBookDelta) -> Result<()> {
        unsafe {
            // Validate header pointer before dereferencing
            if self.header.is_null() {
                return Err(AlphaPulseError::MemoryCorruption);
            }
            
            let header = &mut *self.header;
            
            // Get current write position
            let sequence = header.write_sequence.fetch_add(1, Ordering::AcqRel);
            let index = (sequence % self.capacity as u64) as usize;
            
            // Bounds checking - critical for memory safety
            if index >= self.capacity {
                return Err(AlphaPulseError::BufferOverflow { 
                    index, 
                    capacity: self.capacity 
                });
            }
            
            // Calculate delta position in buffer with bounds validation
            let offset = index * SharedOrderBookDelta::SIZE;
            if self.data_start.is_null() {
                return Err(AlphaPulseError::MemoryCorruption);
            }
            
            let delta_ptr = self.data_start.add(offset) as *mut SharedOrderBookDelta;
            
            // Validate pointer alignment
            if (delta_ptr as usize) % mem::align_of::<SharedOrderBookDelta>() != 0 {
                return Err(AlphaPulseError::MemoryCorruption);
            }
            
            // Write delta with volatile semantics to prevent reordering
            ptr::write_volatile(delta_ptr, *delta);
            
            // Memory fence to ensure write is visible
            std::sync::atomic::fence(Ordering::Release);
            
            // Update cached sequence for readers
            header.cached_write_sequence = sequence + 1;
            
            // Update timestamp with proper error handling
            let now_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(AlphaPulseError::SystemTimeError)?
                .as_nanos() as u64;
            header.last_write_ns.store(now_ns, Ordering::Relaxed);
        }
        
        Ok(())
    }
    
    pub fn get_stats(&self) -> (u64, u64) {
        unsafe {
            let header = &*self.header;
            let writes = header.write_sequence.load(Ordering::Relaxed);
            let last_write = header.last_write_ns.load(Ordering::Relaxed);
            (writes, last_write)
        }
    }
}

// Specialized reader for orderbook deltas
pub struct OrderBookDeltaReader {
    mmap: memmap2::Mmap,
    header: *const RingBufferHeader,
    data_start: *const u8,
    capacity: usize,
    reader_id: usize,
    last_sequence: u64,
}

unsafe impl Send for OrderBookDeltaReader {}
unsafe impl Sync for OrderBookDeltaReader {}

impl OrderBookDeltaReader {
    pub fn open(path: &str, reader_id: usize) -> Result<Self> {
        Self::open_with_retry(path, reader_id, 10, 100)
    }
    
    pub fn open_with_retry(path: &str, reader_id: usize, max_retries: u32, retry_delay_ms: u64) -> Result<Self> {
        if reader_id >= 16 {
            return Err(AlphaPulseError::ConfigError(
                "Reader ID must be less than 16".to_string()
            ));
        }
        
        // Try to open the shared memory file with retries
        let mut retries = 0;
        let file = loop {
            match OpenOptions::new()
                .read(true)
                .open(path) {
                Ok(f) => break f,
                Err(e) if retries < max_retries => {
                    retries += 1;
                    std::thread::sleep(std::time::Duration::from_millis(retry_delay_ms));
                }
                Err(e) => return Err(e.into()),
            }
        };
        
        // Memory map the file (read-only)
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)?
        };
        
        // Get header and validate
        let header_ptr = mmap.as_ptr() as *const RingBufferHeader;
        let capacity = unsafe {
            (*header_ptr).capacity as usize
        };
        
        // Calculate data start position
        let header_size = mem::size_of::<RingBufferHeader>();
        let data_start = unsafe {
            mmap.as_ptr().add(header_size)
        };
        
        // Start from current write position to avoid reading old/overwritten data
        let last_sequence = unsafe {
            (*header_ptr).write_sequence.load(Ordering::Acquire)
        };
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            capacity,
            reader_id,
            last_sequence,
        })
    }
    
    pub fn read_deltas(&mut self) -> Vec<SharedOrderBookDelta> {
        let mut deltas = Vec::new();
        
        unsafe {
            let header = &*self.header;
            
            // Memory fence to see latest writes
            std::sync::atomic::fence(Ordering::Acquire);
            
            // Read the atomic write sequence with proper ordering
            let write_sequence = header.write_sequence.load(Ordering::Acquire);
            
            // Read all deltas since last read
            while self.last_sequence < write_sequence {
                let index = (self.last_sequence % self.capacity as u64) as usize;
                let delta_ptr = self.data_start.add(index * SharedOrderBookDelta::SIZE) as *const SharedOrderBookDelta;
                
                // Read delta with volatile semantics
                let delta = ptr::read_volatile(delta_ptr);
                deltas.push(delta);
                
                self.last_sequence += 1;
            }
            
            // Update reader cursor
            if !deltas.is_empty() {
                header.reader_cursors[self.reader_id].store(
                    self.last_sequence,
                    Ordering::Relaxed
                );
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

pub struct SharedMemoryReader {
    mmap: memmap2::Mmap,
    header: *const RingBufferHeader,
    data_start: *const u8,
    capacity: usize,
    reader_id: usize,
    last_sequence: u64,
}

unsafe impl Send for SharedMemoryReader {}
unsafe impl Sync for SharedMemoryReader {}

impl SharedMemoryReader {
    pub fn open(path: &str, reader_id: usize) -> Result<Self> {
        Self::open_with_retry(path, reader_id, 10, 100)
    }
    
    pub fn open_with_retry(path: &str, reader_id: usize, max_retries: u32, retry_delay_ms: u64) -> Result<Self> {
        if reader_id >= 16 {
            return Err(AlphaPulseError::ConfigError(
                "Reader ID must be less than 16".to_string()
            ));
        }
        
        // Try to open the shared memory file with retries
        let mut retries = 0;
        let file = loop {
            match OpenOptions::new()
                .read(true)
                .open(path) {
                Ok(f) => break f,
                Err(e) if retries < max_retries => {
                    retries += 1;
                    std::thread::sleep(std::time::Duration::from_millis(retry_delay_ms));
                }
                Err(e) => return Err(e.into()),
            }
        };
        
        // Memory map the file (read-only)
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)?
        };
        
        // Get header and validate
        let header_ptr = mmap.as_ptr() as *const RingBufferHeader;
        let capacity = unsafe {
            (*header_ptr).capacity as usize
        };
        
        // Calculate data start position
        let header_size = mem::size_of::<RingBufferHeader>();
        let data_start = unsafe {
            mmap.as_ptr().add(header_size)
        };
        
        // Start from current write position to avoid reading old/overwritten data
        let last_sequence = unsafe {
            (*header_ptr).write_sequence.load(Ordering::Acquire)
        };
        
        Ok(Self {
            mmap,
            header: header_ptr,
            data_start,
            capacity,
            reader_id,
            last_sequence,
        })
    }
    
    pub fn read_trades(&mut self) -> Vec<SharedTrade> {
        let mut trades = Vec::new();
        
        unsafe {
            let header = &*self.header;
            
            // Memory fence to see latest writes
            std::sync::atomic::fence(Ordering::Acquire);
            
            // Read the atomic write sequence with proper ordering
            let write_sequence = header.write_sequence.load(Ordering::Acquire);
            
            // Read all trades since last read
            while self.last_sequence < write_sequence {
                let index = (self.last_sequence % self.capacity as u64) as usize;
                let trade_ptr = self.data_start.add(index * SharedTrade::SIZE) as *const SharedTrade;
                
                // Read trade with volatile semantics
                let trade = ptr::read_volatile(trade_ptr);
                trades.push(trade);
                
                self.last_sequence += 1;
            }
            
            // Update reader cursor
            if !trades.is_empty() {
                header.reader_cursors[self.reader_id].store(
                    self.last_sequence,
                    Ordering::Relaxed
                );
            }
        }
        
        trades
    }
    
    pub fn get_lag(&self) -> u64 {
        unsafe {
            let header = &*self.header;
            let write_sequence = header.write_sequence.load(Ordering::Relaxed);
            write_sequence.saturating_sub(self.last_sequence)
        }
    }
}

// Helper function to clean up shared memory on shutdown
pub fn cleanup_shared_memory(path: &str) -> Result<()> {
    if Path::new(path).exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shared_trade_size() {
        assert_eq!(mem::size_of::<SharedTrade>(), 128);
    }
    
    #[test]
    fn test_ring_buffer_write_read() {
        let path = "/tmp/test_trades";
        cleanup_shared_memory(path).ok();
        
        // Create writer
        let mut writer = SharedMemoryWriter::create(path, 1000).unwrap();
        
        // Write some trades
        for i in 0..10 {
            let trade = SharedTrade::new(
                i as u64,
                "BTC-USD",
                "coinbase",
                50000.0 + i as f64,
                0.1,
                true,
                &format!("trade_{}", i),
            );
            writer.write_trade(&trade).unwrap();
        }
        
        // Create reader and read trades
        let mut reader = SharedMemoryReader::open(path, 0).unwrap();
        let trades = reader.read_trades();
        
        assert_eq!(trades.len(), 10);
        assert_eq!(trades[0].price, 50000.0);
        assert_eq!(trades[9].price, 50009.0);
        
        // Cleanup
        cleanup_shared_memory(path).ok();
    }
}