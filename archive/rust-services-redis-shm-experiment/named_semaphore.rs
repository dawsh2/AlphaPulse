// Named semaphore implementation for macOS
// This uses sem_open which is fully supported on macOS for cross-process communication

use crate::{Result, AlphaPulseError};
use std::ffi::CString;
use std::ptr;
use libc::{sem_t, sem_open, sem_close, sem_unlink, sem_wait, sem_post, sem_trywait, O_CREAT, O_EXCL};
use std::sync::atomic::{AtomicU32, Ordering};
use uuid::Uuid;

static SEMAPHORE_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct NamedSemaphore {
    sem_ptr: *mut sem_t,
    name: CString,
    owned: bool, // Whether we created this semaphore
}

unsafe impl Send for NamedSemaphore {}
unsafe impl Sync for NamedSemaphore {}

impl NamedSemaphore {
    /// Create a new named semaphore
    pub fn create(initial_value: u32) -> Result<Self> {
        // Generate unique name for this semaphore
        let counter = SEMAPHORE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let name = format!("/alphapulse_sem_{}_{}", std::process::id(), counter);
        let c_name = CString::new(name.clone())
            .map_err(|e| AlphaPulseError::ConfigError(format!("Invalid semaphore name: {}", e)))?;
        
        // Create the semaphore with O_CREAT | O_EXCL to ensure it's new
        let sem_ptr = unsafe {
            sem_open(
                c_name.as_ptr(),
                O_CREAT | O_EXCL,
                0o600, // rw for owner only
                initial_value
            )
        };
        
        if sem_ptr == libc::SEM_FAILED {
            return Err(AlphaPulseError::ConfigError(
                format!("sem_open failed: {} (name: {})", std::io::Error::last_os_error(), name)
            ));
        }
        
        Ok(NamedSemaphore {
            sem_ptr,
            name: c_name,
            owned: true,
        })
    }
    
    /// Open an existing named semaphore
    pub fn open(name: &str) -> Result<Self> {
        let c_name = CString::new(name)
            .map_err(|e| AlphaPulseError::ConfigError(format!("Invalid semaphore name: {}", e)))?;
        
        let sem_ptr = unsafe {
            sem_open(
                c_name.as_ptr(),
                0, // No flags - just open existing
                0,
                0
            )
        };
        
        if sem_ptr == libc::SEM_FAILED {
            return Err(AlphaPulseError::ConfigError(
                format!("sem_open failed for {}: {}", name, std::io::Error::last_os_error())
            ));
        }
        
        Ok(NamedSemaphore {
            sem_ptr,
            name: c_name,
            owned: false,
        })
    }
    
    /// Get the name of this semaphore
    pub fn name(&self) -> &str {
        self.name.to_str().unwrap_or("<invalid>")
    }
    
    /// Wait on the semaphore (blocks)
    pub fn wait(&self) -> Result<()> {
        let result = unsafe { sem_wait(self.sem_ptr) };
        if result != 0 {
            return Err(AlphaPulseError::ConfigError(
                format!("sem_wait failed: {}", std::io::Error::last_os_error())
            ));
        }
        Ok(())
    }
    
    /// Post to the semaphore (signal)
    pub fn post(&self) -> Result<()> {
        let result = unsafe { sem_post(self.sem_ptr) };
        if result != 0 {
            return Err(AlphaPulseError::ConfigError(
                format!("sem_post failed: {}", std::io::Error::last_os_error())
            ));
        }
        Ok(())
    }
    
    /// Try to wait without blocking
    pub fn try_wait(&self) -> Result<bool> {
        let result = unsafe { sem_trywait(self.sem_ptr) };
        if result == 0 {
            Ok(true)
        } else {
            let errno = unsafe { *libc::__error() };
            if errno == libc::EAGAIN {
                Ok(false) // Would block
            } else {
                Err(AlphaPulseError::ConfigError(
                    format!("sem_trywait failed: errno={}", errno)
                ))
            }
        }
    }
}

impl Drop for NamedSemaphore {
    fn drop(&mut self) {
        // Close the semaphore
        unsafe {
            sem_close(self.sem_ptr);
        }
        
        // If we created it, unlink it to clean up
        if self.owned {
            unsafe {
                sem_unlink(self.name.as_ptr());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_named_semaphore_basic() {
        let sem = NamedSemaphore::create(0).unwrap();
        println!("Created semaphore: {}", sem.name());
        
        // Post and then wait should work
        sem.post().unwrap();
        sem.wait().unwrap();
    }
    
    #[test]
    fn test_named_semaphore_cross_thread() {
        let sem = Arc::new(NamedSemaphore::create(0).unwrap());
        let sem_clone = Arc::clone(&sem);
        
        // Spawn thread that will wait
        let handle = thread::spawn(move || {
            sem_clone.wait().unwrap();
            42
        });
        
        // Give thread time to start waiting
        thread::sleep(Duration::from_millis(100));
        
        // Post to wake it up
        sem.post().unwrap();
        
        // Thread should complete
        let result = handle.join().unwrap();
        assert_eq!(result, 42);
    }
    
    #[test]
    fn test_named_semaphore_cross_process() {
        // This test would require forking or spawning a subprocess
        // For now, we'll test that we can create and open by name
        let sem1 = NamedSemaphore::create(1).unwrap();
        let name = sem1.name().to_string();
        
        // Open the same semaphore by name
        let sem2 = NamedSemaphore::open(&name).unwrap();
        
        // Both should work
        sem1.wait().unwrap(); // Decrement to 0
        assert!(!sem2.try_wait().unwrap()); // Should block
        sem1.post().unwrap(); // Increment to 1
        assert!(sem2.try_wait().unwrap()); // Should succeed
    }
}